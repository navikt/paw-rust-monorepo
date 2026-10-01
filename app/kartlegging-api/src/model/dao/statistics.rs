use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, FromRow)]
pub(crate) struct KartleggingStatisticsRow {
    pub perioder_total: i64,
    pub perioder_aktiv: i64,
    pub perioder_avsluttet: i64,
    pub ledighet_null: i64,
    pub ledighet_ikke_null: i64,
    pub ledighet_over_0030_dager: i64,
    pub ledighet_over_0060_dager: i64,
    pub ledighet_over_0090_dager: i64,
    pub ledighet_over_0180_dager: i64,
    pub ledighet_over_0365_dager: i64,
    pub ledighet_over_0730_dager: i64,
    pub ledighet_over_1095_dager: i64,
}

#[tracing::instrument(skip_all)]
pub async fn count(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<KartleggingStatisticsRow> {
    tracing::debug!("Count kartlegginger");
    let row = sqlx::query_as::<_, KartleggingStatisticsRow>(
        r#"
        SELECT
            COUNT(*)                                                                                              AS perioder_total,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL)                                                     AS perioder_aktiv,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NOT NULL)                                                 AS perioder_avsluttet,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra IS NULL)                        AS ledighet_null,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra IS NOT NULL)                    AS ledighet_ikke_null,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '30 days')   AS ledighet_over_0030_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '60 days')   AS ledighet_over_0060_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '90 days')   AS ledighet_over_0090_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '180 days')  AS ledighet_over_0180_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '365 days')  AS ledighet_over_0365_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '730 days')  AS ledighet_over_0730_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '1095 days') AS ledighet_over_1095_dager
        FROM kartlegginger;
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}

#[tracing::instrument(skip_all)]
pub async fn count_by_kontor_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id_list: &Vec<String>,
) -> anyhow::Result<KartleggingStatisticsRow> {
    tracing::debug!("Count kartlegginger");
    let row = sqlx::query_as::<_, KartleggingStatisticsRow>(
        r#"
        SELECT
            COUNT(*)                                                                                              AS perioder_total,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL)                                                     AS perioder_aktiv,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NOT NULL)                                                 AS perioder_avsluttet,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra IS NULL)                        AS ledighet_null,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra IS NOT NULL)                    AS ledighet_ikke_null,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '30 days')   AS ledighet_over_0030_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '60 days')   AS ledighet_over_0060_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '90 days')   AS ledighet_over_0090_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '180 days')  AS ledighet_over_0180_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '365 days')  AS ledighet_over_0365_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '730 days')  AS ledighet_over_0730_dager,
            COUNT(*) FILTER (WHERE arbeidssoeker_til IS NULL AND arbeidsledig_fra < NOW() - INTERVAL '1095 days') AS ledighet_over_1095_dager
        FROM arbeidssoekere a
        JOIN kartlegginger k on a.id = k.arbeidssoeker_id
        JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
        WHERE kt.kontor_id = ANY($1);
        "#,
    )
    .bind(kontor_id_list)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}
