use crate::model::dao::kontortilknytning;
use crate::model::dao::kontortilknytning::KontortilknytningRow;
use crate::model::dto::kontortilknytning::KontorType;
use dab_oppfolgingperioder::oppfolgingsperiode::{
    Oppfolgingsperiode, OppfolgingsperiodeAvsluttet, OppfolgingsperiodeEndret,
};
use sqlx::{Postgres, Transaction};

pub async fn lagre_hendelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &'a Oppfolgingsperiode,
) -> anyhow::Result<u64> {
    let rows_affected = match hendelse {
        Oppfolgingsperiode::Startet(data) => upsert_kontortilknytning(tx, data).await?,
        Oppfolgingsperiode::Endret(data) => upsert_kontortilknytning(tx, data).await?,
        Oppfolgingsperiode::Avsluttet(data) => delete_kontortilknytning(tx, data).await?,
    };
    Ok(rows_affected)
}

async fn upsert_kontortilknytning<'a>(
    tx: &mut Transaction<'_, Postgres>,
    data: &'a OppfolgingsperiodeEndret,
) -> anyhow::Result<u64> {
    let row = KontortilknytningRow::new(
        data.oppfolgingsperiode_id.clone(),
        data.aktor_id.clone(),
        data.ident.clone(),
        data.kontor.kontor_id.clone(),
        data.kontor.kontor_navn.clone(),
        KontorType::Arbeidsoppfolging.as_ref().to_string(), // Akkurat nå vil alle kontortilknytninger være av type Arbeidsoppfolging. Feltet åpner for å kunne ta imot andre typer tilknytninger i fremtiden
        data.start_tidspunkt.clone(),
    );
    let count = kontortilknytning::count_by_id(tx, &data.oppfolgingsperiode_id).await?;
    let rows_affected = if count > 1 {
        panic!("Fant flere rader for id ({})", count);
    } else if count == 1 {
        kontortilknytning::update(tx, &row).await?
    } else {
        kontortilknytning::insert(tx, &row).await?
    };
    Ok(rows_affected)
}

async fn delete_kontortilknytning<'a>(
    tx: &mut Transaction<'_, Postgres>,
    data: &'a OppfolgingsperiodeAvsluttet,
) -> anyhow::Result<u64> {
    kontortilknytning::delete(tx, &data.oppfolgingsperiode_id).await
}
