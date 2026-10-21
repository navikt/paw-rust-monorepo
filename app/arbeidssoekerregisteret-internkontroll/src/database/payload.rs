#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "payload_status", rename_all = "snake_case")]
pub enum PayloadStatus {
    Ok,
    Ugyldig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "meldingstype", rename_all = "snake_case")]
pub enum Meldingstype {
    HendelseStartet,
    HendelseAvsluttet,
    HendelseAvvist,
    HendelseAvvistStoppAvPeriode,
    HendelseArbeidssoekerIdFlettetInn,
    HendelseAutomatiskIdMergeIkkeMulig,
    HendelseIdentitetsnummerSammenslaatt,
    HendelseOpplysningerOmArbeidssoekerMottatt,
    BekreftelseHendelseLeveringsfristUtloept,
    BekreftelseHendelseEksternGracePeriodeUtloept,
    BekreftelseHendelseRegisterGracePeriodeUtloept,
    BekreftelseHendelseTilgjengelig,
    BekreftelseHendelseMeldingMottatt,
    BekreftelseHendelsePeriodeAvsluttet,
    BekreftelseHendelseRegisterGracePeriodeGjenstaaendeTid,
    BekreftelseHendelseBaOmAaAvsluttePeriode,
    BekreftelseHendelsePaaVegneAvStartet,
    BekreftelseHendelseGraceUtloeptEtterEksternInnsamling,
    PeriodeStartet,
    PeriodeAvsluttet,
    Opplysninger,
    Profilering,
    Egenvurdering,
    Bekreftelse,
    PaaVegneAvStart,
    PaaVegneAvStopp,
}
