use chrono::DateTime;
use uuid::Uuid;

pub async fn skriv_periode_til_db(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_id: Uuid,
    identitetsnummer: String,
    startet: DateTime<chrono::Utc>,
    brukertype: String
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
            insert into periode (periode_id, identitetsnummer, startet_brukertype, startet_tidspunkt)
            values ($1, $2, $3, $4)
        "#
    ).bind(periode_id)
     .bind(identitetsnummer)
     .bind(startet)
     .bind(brukertype)
     .execute(&mut **tx)
     .await?;
    Ok(())
}

pub async fn avslutt_periode_i_db(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_id: Uuid,
    avsluttet: DateTime<chrono::Utc>,
    brukertype: String
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"
            update periode
            set avsluttet_brukertype = $1, avsluttet_tidspunkt = $2
            where periode_id = $3
        "#
    ).bind(avsluttet)
     .bind(brukertype)
     .bind(periode_id)
     .execute(&mut **tx)
     .await?;
    Ok(())
}

