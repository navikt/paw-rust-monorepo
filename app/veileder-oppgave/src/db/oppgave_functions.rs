use crate::db::oppgave_hendelse_logg_row::{OppgaveHendelseLoggBatchRow, OppgaveHendelseLoggRow};
use crate::db::oppgave_row::OppgaveRow;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;
use std::num::NonZeroU32;
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
use crate::domain::ekstern_oppgave_id::EksternOppgaveId;
use crate::domain::oppgave_id::OppgaveId;

pub async fn hent_nyeste_oppgave(
    arbeidssoeker_id: ArbeidssoekerId,
    oppgave_type: OppgaveType,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<Oppgave>> {
    let oppgave_row = match hent_nyeste_oppgave_for_arbeidssoeker(arbeidssoeker_id, &oppgave_type, tx).await? {
        None => return Ok(None),
        Some(row) => row,
    };

    let oppgave_id = OppgaveId::from(oppgave_row.id);
    let hendelse_logg: Vec<HendelseLoggEntry> = hent_hendelse_logg(oppgave_id, tx).await?;
    let oppgave = Oppgave::fra_db(
        oppgave_id,
        oppgave_row.melding_id,
        oppgave_row.type_.parse()?,
        oppgave_row.status.parse()?,
        oppgave_row.opplysninger,
        ArbeidssoekerId::from(oppgave_row.arbeidssoeker_id),
        Identitetsnummer::new(oppgave_row.identitetsnummer)
            .expect("Identitetsnummer fra database er ugyldig — dataintegritetsfeil"),
        oppgave_row.ekstern_oppgave_id.map(EksternOppgaveId::from),
        oppgave_row.tidspunkt,
        hendelse_logg,
    );

    Ok(Some(oppgave))
}

async fn hent_nyeste_oppgave_for_arbeidssoeker(
    arbeidssoeker_id: ArbeidssoekerId,
    oppgave_type: &OppgaveType,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<OppgaveRow>> {
    let rows = sqlx::query_as::<_, OppgaveRow>(
        r#"
        SELECT
            id,
            melding_id,
            type AS type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgaver
        WHERE arbeidssoeker_id = $1
          AND type = $2
        ORDER BY id DESC
        "#,
    )
    .bind(i64::from(arbeidssoeker_id))
    .bind(oppgave_type.to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(rows)
}

pub async fn finn_oppgave_for_ekstern_id(
    ekstern_id: EksternOppgaveId,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<Oppgave>> {
    let rows = sqlx::query_as::<_, OppgaveRow>(
        r#"
        SELECT
            id,
            melding_id,
            type AS type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgaver
        WHERE ekstern_oppgave_id = $1
        "#,
    )
    .bind(i64::from(ekstern_id))
    .fetch_optional(&mut **tx)
    .await?;

    let oppgave_row = match rows {
        None => return Ok(None),
        Some(row) => row,
    };

    let oppgave_id = OppgaveId::from(oppgave_row.id);
    let hendelse_logg: Vec<HendelseLoggEntry> = hent_hendelse_logg(oppgave_id, tx).await?;
    let oppgave = Oppgave::fra_db(
        oppgave_id,
        oppgave_row.melding_id,
        oppgave_row.type_.parse()?,
        oppgave_row.status.parse()?,
        oppgave_row.opplysninger,
        ArbeidssoekerId::from(oppgave_row.arbeidssoeker_id),
        Identitetsnummer::new(oppgave_row.identitetsnummer)
            .expect("Identitetsnummer fra database er ugyldig — dataintegritetsfeil"),
        oppgave_row.ekstern_oppgave_id.map(EksternOppgaveId::from),
        oppgave_row.tidspunkt,
        hendelse_logg,
    );

    Ok(Some(oppgave))
}

async fn hent_hendelse_logg(
    oppgave_id: OppgaveId,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<HendelseLoggEntry>> {
    let rows = sqlx::query_as::<_, OppgaveHendelseLoggRow>(
        r#"
        SELECT
            status,
            melding,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgave_hendelse_logg
        WHERE oppgave_id = $1
        ORDER BY tidspunkt DESC
        "#,
    )
    .bind(i64::from(oppgave_id))
    .fetch_all(&mut **transaction)
    .await?;

    let hendelse_logg: Vec<HendelseLoggEntry> = rows
        .into_iter()
        .map(|row| {
            let status = row.status.parse()?;
            Ok(HendelseLoggEntry::new(status, row.melding, row.tidspunkt))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(hendelse_logg)
}

pub async fn lagre_oppgave(
    oppgave: &Oppgave,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<OppgaveId> {
    let oppgave_id = sqlx::query_scalar(
        r#"
        INSERT INTO oppgaver (type, status, melding_id, opplysninger, arbeidssoeker_id, identitetsnummer, tidspunkt)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(oppgave.type_.to_string())
    .bind(oppgave.status.to_string())
    .bind(oppgave.melding_id)
    .bind(&oppgave.opplysninger)
    .bind(i64::from(oppgave.arbeidssoeker_id))
    .bind(String::from(oppgave.identitetsnummer.clone()))
    .bind(oppgave.tidspunkt)
    .fetch_one(&mut **tx)
    .await?;

    Ok(OppgaveId(oppgave_id))
}

pub async fn oppdater_hendelse_logg(
    oppgave_id: OppgaveId,
    entry: HendelseLoggEntry,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        INSERT INTO oppgave_hendelse_logg (oppgave_id, status, melding, tidspunkt)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(i64::from(oppgave_id))
    .bind(entry.status.to_string())
    .bind(&entry.melding)
    .bind(entry.tidspunkt)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected())
}

pub async fn oppdater_oppgave_med_ekstern_id(
    oppgave_id: OppgaveId,
    ekstern_oppgave_id: EksternOppgaveId,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE oppgaver
        SET ekstern_oppgave_id = $1
        WHERE id = $2
        "#,
    )
    .bind(i64::from(ekstern_oppgave_id))
    .bind(i64::from(oppgave_id))
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn bytt_oppgave_status(
    oppgave_id: OppgaveId,
    expected_status: OppgaveStatus,
    new_status: OppgaveStatus,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE oppgaver
        SET status = $1
        WHERE id = $2
          AND status = $3
        "#,
    )
    .bind(new_status.to_string())
    .bind(i64::from(oppgave_id))
    .bind(expected_status.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn hent_de_eldste_ubehandlede_oppgavene(
    antall_oppgaver: NonZeroU32,
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<Oppgave>> {
    let oppgave_rows = sqlx::query_as::<_, OppgaveRow>(
        r#"
        SELECT
            id,
            melding_id,
            type AS type_,
            status,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            ekstern_oppgave_id,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgaver
        WHERE status = $1
            AND ekstern_oppgave_id IS NULL
            AND tidspunkt >= $2
        ORDER BY tidspunkt ASC
        LIMIT $3
        "#,
    )
    .bind(OppgaveStatus::Ubehandlet.to_string())
    .bind(fra_tidspunkt)
    .bind(antall_oppgaver.get() as i64)
    .fetch_all(&mut **transaction)
    .await?;

    let oppgave_ider: Vec<i64> = oppgave_rows.iter().map(|r| r.id).collect();
    let mut hendelse_logg_map = hent_hendelse_logger(&oppgave_ider, transaction).await?;

    let mut oppgaver = Vec::with_capacity(oppgave_rows.len());
    for oppgave_row in oppgave_rows {
        let hendelse_logg = hendelse_logg_map.remove(&oppgave_row.id).unwrap_or_default();
        let oppgave = Oppgave::fra_db(
            OppgaveId::from(oppgave_row.id),
            oppgave_row.melding_id,
            oppgave_row.type_.parse()?,
            oppgave_row.status.parse()?,
            oppgave_row.opplysninger,
            ArbeidssoekerId::from(oppgave_row.arbeidssoeker_id),
            Identitetsnummer::new(oppgave_row.identitetsnummer)
                .expect("Identitetsnummer fra database er ugyldig — dataintegritetsfeil"),
            oppgave_row.ekstern_oppgave_id.map(EksternOppgaveId::from),
            oppgave_row.tidspunkt,
            hendelse_logg,
        );
        oppgaver.push(oppgave);
    }

    Ok(oppgaver)
}

async fn hent_hendelse_logger(
    oppgave_ider: &[i64],
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<HashMap<i64, Vec<HendelseLoggEntry>>> {
    let rows = sqlx::query_as::<_, OppgaveHendelseLoggBatchRow>(
        r#"
        SELECT
            oppgave_id,
            status,
            melding,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgave_hendelse_logg
        WHERE oppgave_id = ANY($1)
        ORDER BY oppgave_id, tidspunkt DESC
        "#,
    )
        .bind(oppgave_ider)
        .fetch_all(&mut **transaction)
        .await?;

    let mut map: HashMap<i64, Vec<HendelseLoggEntry>> = HashMap::new();
    for row in rows {
        let status = row.status.parse()?;
        let entry = HendelseLoggEntry::new(status, row.melding, row.tidspunkt);
        map.entry(row.oppgave_id).or_default().push(entry);
    }
    Ok(map)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oppgave::Oppgave;
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_status::OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
    use crate::domain::oppgave_type::OppgaveType;
    use chrono::Utc;
    use paw_test::setup_test_db::setup_test_db;
    use uuid::Uuid;
    use OppgaveType::AvvistUnder18;
    use types::arbeidssoeker_id::ArbeidssoekerId;
    use types::identitetsnummer::Identitetsnummer;

    #[tokio::test]
    async fn test_hent_de_eldste_ubehandlede_oppgavene() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let id = ArbeidssoekerId(1234567);
        let eldste_oppgave = test_oppgave(id, Ubehandlet, Utc::now() - chrono::Duration::days(2));
        let nest_eldste_oppgave = test_oppgave(id, Ubehandlet, Utc::now() - chrono::Duration::days(1));
        let irrelevant_oppgave = test_oppgave(id, Ferdigbehandlet, Utc::now() - chrono::Duration::days(1337));
        let yngste_oppgave = test_oppgave(id, Ubehandlet, Utc::now());

        lagre_oppgave(&eldste_oppgave, &mut tx).await?;
        lagre_oppgave(&nest_eldste_oppgave, &mut tx).await?;
        lagre_oppgave(&irrelevant_oppgave, &mut tx).await?;
        lagre_oppgave(&yngste_oppgave, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let antall_oppgaver = NonZeroU32::new(2).unwrap();
        let fra_tidspunkt = Utc::now() - chrono::Duration::days(1336);
        let oppgaver =
            hent_de_eldste_ubehandlede_oppgavene(antall_oppgaver, fra_tidspunkt, &mut tx).await?;

        assert_eq!(oppgaver.len(), antall_oppgaver.get() as usize);
        let eldste = &oppgaver[0];
        let nest_eldste = &oppgaver[1];
        assert!(eldste.tidspunkt < nest_eldste.tidspunkt);
        assert_eq!(eldste.status, Ubehandlet);
        assert_eq!(nest_eldste.status, Ubehandlet);
        assert!(eldste.tidspunkt < yngste_oppgave.tidspunkt);
        assert!(nest_eldste.tidspunkt < yngste_oppgave.tidspunkt);
        Ok(())
    }

    #[tokio::test]
    async fn test_fra_tidspunkt_filtrerer_bort_gamle_oppgaver() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let id = ArbeidssoekerId(1234567);
        let now = Utc::now();
        let gammel_oppgave = test_oppgave(id, Ubehandlet, now - chrono::Duration::seconds(2));
        let ny_oppgave = test_oppgave(id, Ubehandlet, now);

        lagre_oppgave(&gammel_oppgave, &mut tx).await?;
        lagre_oppgave(&ny_oppgave, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let fra_tidspunkt = now - chrono::Duration::seconds(1);
        let antall_oppgaver = NonZeroU32::new(10).unwrap();
        let oppgaver = hent_de_eldste_ubehandlede_oppgavene(antall_oppgaver, fra_tidspunkt, &mut tx).await?;
        assert_eq!(oppgaver.len(), 1, "Skal bare finne ny_oppgave");
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let fra_tidspunkt = now + chrono::Duration::seconds(1);
        let oppgaver = hent_de_eldste_ubehandlede_oppgavene(antall_oppgaver, fra_tidspunkt, &mut tx).await?;
        assert_eq!(
            oppgaver.len(),
            0,
            "Skal ikke finne noen oppgaver med fra_tidspunkt i fremtiden"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_oppdater_ekstern_id() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let arbeidssoeker_id = ArbeidssoekerId(12345);
        let oppgave_id = lagre_oppgave(&test_oppgave(arbeidssoeker_id, Ubehandlet, Utc::now()), &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let ekstern_oppgave_id = EksternOppgaveId::from(1337);
        let oppdatert =
            oppdater_oppgave_med_ekstern_id(oppgave_id, ekstern_oppgave_id, &mut tx).await?;
        tx.commit().await?;
        assert!(oppdatert);

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, AvvistUnder18, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.ekstern_oppgave_id.unwrap(), ekstern_oppgave_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_bytt_oppgave_status() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let arbeidssoeker_id = ArbeidssoekerId(12345);
        let oppgave_id = lagre_oppgave(&test_oppgave(arbeidssoeker_id, Ubehandlet, Utc::now()), &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppdatert = bytt_oppgave_status(oppgave_id, Ubehandlet, Ferdigbehandlet, &mut tx).await?;
        tx.commit().await?;
        assert!(oppdatert);

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, AvvistUnder18, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, Ferdigbehandlet);

        Ok(())
    }

    #[tokio::test]
    async fn lagre_oppgave_returnerer_oppgave_id() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let arbeidssoeker_id = ArbeidssoekerId(12345);
        let oppgave_id = lagre_oppgave(&test_oppgave(arbeidssoeker_id, Ubehandlet, Utc::now()), &mut tx).await?;
        tx.commit().await?;
        assert_eq!(oppgave_id, OppgaveId(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_hent_nyeste_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = ArbeidssoekerId(12345);
        lagre_oppgave(&test_oppgave(arbeidssoeker_id, Ubehandlet, Utc::now()), &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, AvvistUnder18, &mut tx).await?;
        tx.commit().await?;

        let oppgave = oppgave.unwrap();
        assert_eq!(oppgave.type_, AvvistUnder18);
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.arbeidssoeker_id, ArbeidssoekerId(12345));
        assert_eq!(oppgave.identitetsnummer, Identitetsnummer::new("12345678901".to_string()).unwrap());
        assert_eq!(
            oppgave.opplysninger,
            vec!["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
        );

        let mut tx = pg_pool.begin().await?;
        assert_eq!(hent_nyeste_oppgave(ArbeidssoekerId(99999), AvvistUnder18, &mut tx).await?, None);

        Ok(())
    }

    fn test_oppgave(
        arbeidssoeker_id: ArbeidssoekerId,
        status: OppgaveStatus,
        tidspunkt: DateTime<Utc>,
    ) -> Oppgave {
        Oppgave::new(
            Uuid::new_v4(),
            AvvistUnder18,
            status,
            vec!["ER_UNDER_18_AAR".to_string(), "BOSATT_ETTER_FREG_LOVEN".to_string()],
            arbeidssoeker_id,
            Identitetsnummer::new("12345678901".to_string()).unwrap(),
            tidspunkt,
        )
    }
}
