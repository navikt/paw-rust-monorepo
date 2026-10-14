/// Operation type of a trace, stored as `trace_operation_type.operation_type`.
/// The numbers are persisted: never reuse or renumber a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum TraceOperationType {
    PeriodeStartet = 1,
    PeriodeStoppet = 2,
    OpplysningerLevert = 3,
    BekreftelseLevert = 4,
    BekreftelseIkkeLevert = 5,
    PaaVegneAvStartet = 6,
    PaaVegneAvStoppet = 7,
    EgenvurderingLevert = 8,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown trace operation type: {0}")]
pub struct UnknownTraceOperationType(pub i16);

impl From<TraceOperationType> for i16 {
    fn from(value: TraceOperationType) -> Self {
        value as i16
    }
}

impl TryFrom<i16> for TraceOperationType {
    type Error = UnknownTraceOperationType;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::PeriodeStartet),
            2 => Ok(Self::PeriodeStoppet),
            3 => Ok(Self::OpplysningerLevert),
            4 => Ok(Self::BekreftelseLevert),
            5 => Ok(Self::BekreftelseIkkeLevert),
            6 => Ok(Self::PaaVegneAvStartet),
            7 => Ok(Self::PaaVegneAvStoppet),
            8 => Ok(Self::EgenvurderingLevert),
            other => Err(UnknownTraceOperationType(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdiene_er_faste() {
        let expected = [
            (TraceOperationType::PeriodeStartet, 1),
            (TraceOperationType::PeriodeStoppet, 2),
            (TraceOperationType::OpplysningerLevert, 3),
            (TraceOperationType::BekreftelseLevert, 4),
            (TraceOperationType::BekreftelseIkkeLevert, 5),
            (TraceOperationType::PaaVegneAvStartet, 6),
            (TraceOperationType::PaaVegneAvStoppet, 7),
            (TraceOperationType::EgenvurderingLevert, 8),
        ];
        for (operation_type, value) in expected {
            assert_eq!(i16::from(operation_type), value);
            assert_eq!(TraceOperationType::try_from(value), Ok(operation_type));
        }
    }

    #[test]
    fn ukjent_verdi_gir_feil() {
        assert_eq!(
            TraceOperationType::try_from(0),
            Err(UnknownTraceOperationType(0))
        );
        assert_eq!(
            TraceOperationType::try_from(9),
            Err(UnknownTraceOperationType(9))
        );
    }
}
