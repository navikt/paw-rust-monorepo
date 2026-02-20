use crate::kafka::periode_deserializer::{BrukerType, Periode};
use crate::vo::kilde::InfoKilde;
use crate::vo::status;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::Startet;
use sqlx::{Postgres, Transaction};
use status::Status;

pub async fn opprett_aktiv_periode(
    tx: &mut Transaction<'_, Postgres>,
    periode: &Periode,
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
        INSERT INTO perioder (
          id,
          periode_aktiv,
          periode_startet_timestamp,
          periode_startet_brukertype,
          sist_oppdatert_timestamp,
          sist_oppdatert_status
      ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(periode.id)
    .bind(true)
    .bind(periode.startet.tidspunkt)
    .bind(periode.startet.utfoert_av.bruker_type.to_string())
    .bind(chrono::Utc::now())
    .bind(Status::Ok.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn avslutt_periode(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &uuid::Uuid,
    avsluttet_timestamp: &chrono::DateTime<chrono::Utc>,
    avsluttet_brukertype: &BrukerType,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        UPDATE perioder
        SET periode_aktiv = $1,
            periode_avsluttet_timestamp = $2,
            periode_avsluttet_brukertype = $3
        WHERE id = $4
        "#,
    )
    .bind(false)
    .bind(avsluttet_timestamp)
    .bind(avsluttet_brukertype.to_string())
    .bind(periode_id)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn skrive_startet_hendelse(
    tx: &mut Transaction<'_, Postgres>,
    startet: &Startet,
    kafka_record_key: i64,
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
                insert into periode_metadata (
                    periode_id,
                    identitetsnummer,
                    arbeidssoeker_id,
                    kafka_key
            ) values ($1, $2, $3, $4)
        "#,
    )
    .bind(startet.hendelse_id)
    .bind(startet.identitetsnummer.clone())
    .bind(startet.id)
    .bind(kafka_record_key)
    .execute(&mut **tx)
    .await?;
    let _ = sqlx::query(
        r#"
                insert into options (
                    periode_id,
                    kilde,
                    tidspunkt,
                    opplysninger
            ) values ($1, $2, $3, $4)
        "#,
    )
    .bind(startet.hendelse_id)
    .bind(InfoKilde::StartetHendelse.to_string())
    .bind(startet.metadata.tidspunkt)
    .bind(
        startet
            .opplysninger
            .iter()
            .map(|o| o.to_string())
            .collect::<Vec<String>>(),
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn skriv_pdl_info(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &uuid::Uuid,
    pdl_info: Vec<Opplysning>,
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
                insert into options (
                    periode_id,
                    kilde,
                    tidspunkt,
                    opplysninger
            ) values ($1, $2, $3, $4)
        "#,
    )
    .bind(periode_id)
    .bind(InfoKilde::PdlSjekk.to_string())
    .bind(chrono::Utc::now())
    .bind(
        pdl_info
            .iter()
            .map(|o| o.to_string())
            .collect::<Vec<String>>(),
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
