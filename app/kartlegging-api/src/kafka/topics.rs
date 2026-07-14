use dab_oppfolgingperioder::oppfolgingsperiode::POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
use eksterne_hendelser::opplysninger::PAW_OPPLYSNINGER_TOPIC;
use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
use eksterne_hendelser::profilering::PAW_PROFILERING_TOPIC;

pub static TOPICS: [&str; 7] = [
    PAW_PERIODE_TOPIC,
    PAW_OPPLYSNINGER_TOPIC,
    PAW_PROFILERING_TOPIC,
    PAW_EGENVURDERING_TOPIC,
    PAW_BEKREFTELSE_TOPIC,
    PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC,
    POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC,
];
