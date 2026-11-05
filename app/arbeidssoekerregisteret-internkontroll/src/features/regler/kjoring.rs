use std::sync::Arc;

use paw_kafka::hwm::stream::assigned_partitions::AssignedPartitions;
use paw_observability::health::simple_app_state::AppState;
use sqlx::{PgConnection, PgPool};
use tracing::error;

use crate::features::regler::{
    batch::neste_batch,
    config::RegelConfig,
    fremdrift::{Fremdrift, oppdater_fremdrift},
    funn::lagre_vurdering,
    regel::RegistrertRegel,
};

pub struct Jobb<'a> {
    pub regel: &'a RegistrertRegel,
    pub partisjon: i32,
}

/// En jobb med fremdriften den starter fra. `fremdrift` har én rad per topic
/// regelen leser, i samme rekkefølge som `regel.topic_ids`. Topics som ikke er
/// lest ennå, har `siste_offset` -1.
pub struct JobbMedFremdrift<'a> {
    pub regel: &'a RegistrertRegel,
    pub partisjon: i32,
    pub fremdrift: Vec<Fremdrift>,
}

pub fn spawn_regelkjoring(
    pool: PgPool,
    regler: Vec<RegistrertRegel>,
    tildelt: AssignedPartitions,
    config: RegelConfig,
    app_state: Arc<AppState>,
) -> tokio::task::JoinHandle<Result<(), sqlx::Error>> {
    tokio::spawn(async move {
        let resultat = regelloop(&pool, &regler, &tildelt, &config, &app_state).await;
        if let Err(e) = &resultat {
            error!("Regelkjøring stoppet: {e}");
            app_state.set_is_alive(false);
        }
        resultat
    })
}

async fn regelloop(
    pool: &PgPool,
    regler: &[RegistrertRegel],
    tildelt: &AssignedPartitions,
    config: &RegelConfig,
    app_state: &AppState,
) -> Result<(), sqlx::Error> {
    while app_state.is_alive() {
        if kjor_runde(pool, regler, tildelt, config.batch_size).await? == 0 {
            tokio::time::sleep(config.idle_sleep()).await;
        }
    }
    Ok(())
}

/// Én runde: én batch per regel og partisjon vi har reservert, i én transaksjon.
pub async fn kjor_runde(
    pool: &PgPool,
    regler: &[RegistrertRegel],
    tildelt: &AssignedPartitions,
    batch_size: i64,
) -> Result<usize, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let reserverte = reserver_partisjoner(&mut tx, &tildelt.partitions()).await?;
    let jobber = finn_jobber(regler, &reserverte);
    let mut jobber = hent_fremdrift_for(&mut tx, jobber).await?;
    let mut behandlet = 0;
    for jobb in &mut jobber {
        behandlet += kjor_jobb(&mut tx, jobb, batch_size).await?;
    }
    tx.commit().await?;
    Ok(behandlet)
}

/// Låser partisjonene for regelkjøring ut transaksjonen. Partisjoner en annen
/// pod holder, blir ikke med.
///
/// Låsene slippes ved commit, rollback og brutt forbindelse. Radene settes inn
/// og låses i fast rekkefølge, så to pods kan ikke vente på hverandre i kryss.
pub async fn reserver_partisjoner(
    conn: &mut PgConnection,
    partisjoner: &[i32],
) -> Result<Vec<i32>, sqlx::Error> {
    if partisjoner.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query(
        "INSERT INTO regel_partisjon_laas (topic_partition)
         SELECT p FROM UNNEST($1::INTEGER[]) AS p ORDER BY p
         ON CONFLICT DO NOTHING",
    )
    .bind(partisjoner)
    .execute(&mut *conn)
    .await?;
    sqlx::query_scalar(
        "SELECT topic_partition
         FROM regel_partisjon_laas
         WHERE topic_partition = ANY($1)
         ORDER BY topic_partition
         FOR UPDATE SKIP LOCKED",
    )
    .bind(partisjoner)
    .fetch_all(conn)
    .await
}

pub fn finn_jobber<'a>(regler: &'a [RegistrertRegel], partisjoner: &[i32]) -> Vec<Jobb<'a>> {
    regler
        .iter()
        .flat_map(|regel| {
            partisjoner
                .iter()
                .map(move |&partisjon| Jobb { regel, partisjon })
        })
        .collect()
}

pub async fn hent_fremdrift_for<'a>(
    conn: &mut PgConnection,
    jobber: Vec<Jobb<'a>>,
) -> Result<Vec<JobbMedFremdrift<'a>>, sqlx::Error> {
    let regler: Vec<i16> = jobber.iter().map(|j| j.regel.regel.id() as i16).collect();
    let versjoner: Vec<_> = jobber.iter().map(|j| j.regel.regel.versjon()).collect();
    let partisjoner: Vec<i32> = jobber.iter().map(|j| j.partisjon).collect();
    let lagret: Vec<Fremdrift> = sqlx::query_as(
        "SELECT f.regel, f.regelversjon, f.topic_id, f.topic_partition, f.siste_offset, f.siste_stream_ms
         FROM regel_fremdrift f
         JOIN UNNEST($1::SMALLINT[], $2::INTEGER[], $3::INTEGER[])
              AS j(regel, regelversjon, topic_partition)
           USING (regel, regelversjon, topic_partition)",
    )
    .bind(regler)
    .bind(versjoner)
    .bind(partisjoner)
    .fetch_all(conn)
    .await?;
    Ok(jobber
        .into_iter()
        .map(|jobb| {
            let regel = jobb.regel.regel.as_ref();
            let fremdrift = jobb
                .regel
                .topic_ids
                .iter()
                .map(|&topic_id| {
                    lagret
                        .iter()
                        .find(|f| {
                            f.regel == regel.id()
                                && f.regelversjon == regel.versjon()
                                && f.topic_partition == jobb.partisjon
                                && f.topic_id == topic_id
                        })
                        .copied()
                        .unwrap_or(Fremdrift {
                            regel: regel.id(),
                            regelversjon: regel.versjon(),
                            topic_id,
                            topic_partition: jobb.partisjon,
                            siste_offset: -1,
                            siste_stream_ms: None,
                        })
                })
                .collect();
            JobbMedFremdrift {
                regel: jobb.regel,
                partisjon: jobb.partisjon,
                fremdrift,
            }
        })
        .collect())
}

/// Kjører én batch for jobben og flytter `jobb.fremdrift` fram.
pub async fn kjor_jobb(
    conn: &mut PgConnection,
    jobb: &mut JobbMedFremdrift<'_>,
    batch_size: i64,
) -> Result<usize, sqlx::Error> {
    let regel = jobb.regel.regel.as_ref();
    let batch = neste_batch(conn, jobb.partisjon, &jobb.fremdrift, batch_size).await?;
    for rad in &batch {
        let vurdering = regel.vurder(conn, rad, &jobb.fremdrift).await?;
        lagre_vurdering(
            conn,
            regel.id(),
            regel.versjon(),
            regel.nivaa(),
            rad.record_metadata_id,
            vurdering,
        )
        .await?;
        let fremdrift = jobb
            .fremdrift
            .iter_mut()
            .find(|f| f.topic_id == rad.topic_id)
            .expect("neste_batch gir bare rader fra topics i fremdriften");
        fremdrift.siste_offset = rad.record_offset;
        fremdrift.siste_stream_ms = rad.stream_ms;
    }
    let lest: Vec<Fremdrift> = jobb
        .fremdrift
        .iter()
        .filter(|f| f.siste_offset >= 0)
        .copied()
        .collect();
    oppdater_fremdrift(conn, &lest).await?;
    Ok(batch.len())
}
