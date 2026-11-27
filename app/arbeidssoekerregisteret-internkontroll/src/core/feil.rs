use std::error::Error;

/// Gjør en feil om til en ny feil med bare `Display`-teksten. `Debug` på
/// sqlx-feil tar med `detail` fra Postgres, som ved brudd på NOT NULL og CHECK
/// inneholder hele raden, også fødselsnummer. Bruk denne før en feil kan bli
/// skrevet ut med `{:?}`, som når `main` returnerer `Err`.
pub fn uten_detaljer(feil: Box<dyn Error>) -> Box<dyn Error> {
    feil.to_string().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test]
    async fn debug_viser_ikke_raden_fra_postgres() {
        let pg = setup_postgres_container().await.unwrap();
        let mut conn = pg.pg_pool.acquire().await.unwrap();
        sqlx::query("CREATE TEMP TABLE t (fnr TEXT CHECK (fnr = ''))")
            .execute(&mut *conn)
            .await
            .unwrap();
        let feil = sqlx::query("INSERT INTO t VALUES ('12345678901')")
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert!(
            format!("{feil:?}").contains("12345678901"),
            "testen må gi en rad i detail"
        );

        let trygg = uten_detaljer(feil.into());
        assert!(!format!("{trygg:?}").contains("12345678901"), "{trygg:?}");
        assert!(
            trygg.to_string().contains("violates check constraint"),
            "{trygg}"
        );
    }
}
