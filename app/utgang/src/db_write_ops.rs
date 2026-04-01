use crate::kafka::periode_deserializer::{BrukerType, Periode};
use crate::vo::kilde::InfoKilde;
use crate::vo::status::Status;
use interne_hendelser::Startet;
use interne_hendelser::vo::Opplysning;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn opprett_aktiv_periode(
    tx: &mut Transaction<'_, Postgres>,
    periode: &Periode,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        INSERT INTO periode (
          id,
          periode_aktiv,
          periode_startet_timestamp,
          periode_startet_brukertype,
          sist_oppdatert_timestamp,
          sist_oppdatert_status
      ) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING
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
    Ok(res.rows_affected() == 1)
}

pub async fn skriv_status(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &uuid::Uuid,
    status: &Status,
    tidspunkt: &chrono::DateTime<chrono::Utc>,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        UPDATE periode
        SET sist_oppdatert_timestamp = $1,
            sist_oppdatert_status = $2
        WHERE id = $3
        "#,
    )
    .bind(tidspunkt)
    .bind(status.to_string())
    .bind(periode_id)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn skriv_status_batch(
    tx: &mut Transaction<'_, Postgres>,
    periode_ids: &[Uuid],
    status: &Status,
    tidspunkt: &chrono::DateTime<chrono::Utc>,
) -> Result<u64, sqlx::Error> {
    if periode_ids.is_empty() {
        return Ok(0);
    }
    let res = sqlx::query(
        r#"
        UPDATE periode
        SET sist_oppdatert_timestamp = $1,
            sist_oppdatert_status = $2
        WHERE id = ANY($3::uuid[])
        "#,
    )
    .bind(tidspunkt)
    .bind(status.to_string())
    .bind(periode_ids)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected())
}

pub async fn avslutt_periode(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &uuid::Uuid,
    avsluttet_timestamp: &chrono::DateTime<chrono::Utc>,
    avsluttet_brukertype: &BrukerType,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        UPDATE periode
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
                insert into opplysninger (
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
    tracing::info!("Startet hendelse lagret i databsen");
    Ok(())
}

pub async fn skriv_pdl_info(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &uuid::Uuid,
    pdl_info: Vec<Opplysning>,
) -> Result<(), sqlx::Error> {
    let opplysninger_id: i64 = sqlx::query_scalar(
        r#"
                insert into opplysninger (
                    periode_id,
                    kilde,
                    tidspunkt,
                    opplysninger
            ) values ($1, $2, $3, $4)
            RETURNING id
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
    .fetch_one(&mut **tx)
    .await?;
    klar_for_kontrol(tx, opplysninger_id).await?;
    Ok(())
}

pub async fn klar_for_kontrol(
    tx: &mut Transaction<'_, Postgres>,
    opplysninger_id: i64,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        INSERT INTO klar_for_kontroll (opplysninger_id)
        VALUES ($1)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(opplysninger_id)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn ferdig_kontrollert(
    tx: &mut Transaction<'_, Postgres>,
    opplysninger_id: i64,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        delete from klar_for_kontroll
        where opplysninger_id = $1
        "#,
    )
    .bind(opplysninger_id)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn skriv_pdl_info_batch(
    tx: &mut Transaction<'_, Postgres>,
    batch: Vec<(Uuid, Vec<Opplysning>)>,
) -> Result<(), sqlx::Error> {
    if batch.is_empty() {
        return Ok(());
    }

    let tidspunkt = chrono::Utc::now();
    let kilde = InfoKilde::PdlSjekk.to_string();

    let periode_ids: Vec<Uuid> = batch.iter().map(|(id, _)| *id).collect();
    let opplysninger_json: Vec<String> = batch
        .iter()
        .map(|(_, ops)| {
            let strings: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
            serde_json::to_string(&strings)
                .map_err(|e| sqlx::Error::Protocol(format!("opplysninger serialization failed: {e}")))
        })
        .collect::<Result<_, _>>()?;

    let ids: Vec<i64> = sqlx::query_scalar(
        r#"
        INSERT INTO opplysninger (periode_id, kilde, tidspunkt, opplysninger)
        SELECT
            periode_id,
            $2,
            $3,
            ARRAY(SELECT jsonb_array_elements_text(ops::jsonb))
        FROM UNNEST($1::uuid[], $4::text[]) AS t(periode_id, ops)
        RETURNING id
        "#,
    )
    .bind(&periode_ids)
    .bind(&kilde)
    .bind(tidspunkt)
    .bind(&opplysninger_json)
    .fetch_all(&mut **tx)
    .await?;

    klar_for_kontroll_batch(tx, &ids).await
}

pub async fn klar_for_kontroll_batch(
    tx: &mut Transaction<'_, Postgres>,
    opplysninger_ids: &[i64],
) -> Result<(), sqlx::Error> {
    if opplysninger_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO klar_for_kontroll (opplysninger_id)
        SELECT * FROM UNNEST($1::bigint[])
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(opplysninger_ids)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

