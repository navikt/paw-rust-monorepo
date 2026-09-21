# Recalculating `arbeidsledig_fra`

## Problem

`arbeidsledig_fra` is derived in `bekreftelse_process.rs`, but the rule has changed and existing rows must be recalculated more than once. The selected approach is a Flyway migration that clears the field and rewinds only the bekreftelse topic HWM, not a script or CLI.

## Current `arbeidsledig_fra` calculation

### 1) Bekreftelse event updates existing kartlegging

In `BekreftelseProcessor::utled_arbeidsledighet_fra_bekreftelse`:

- If `har_jobbet_i_denne_perioden == true`: set `arbeidsledig_fra = NULL`.
- If `har_jobbet_i_denne_perioden == false` and current `kartlegging.arbeidsledig_fra` is `NULL`: currently set to `bekreftelse.svar.gjelder_fra`.
- If `har_jobbet_i_denne_perioden == false` and current `kartlegging.arbeidsledig_fra` already has a value: keep existing value unchanged.

This means the first "not worked" confirmation in a period sets the value, later "not worked" confirmations preserve it, and any "worked" confirmation clears it.

## Updated rule to implement for initial population

When processing a bekreftelse where `har_jobbet_i_denne_perioden == false` and `kartlegging.arbeidsledig_fra` is not yet set, apply these rules against `arbeidssoeker_fra` (`periode.startet_tidspunkt`):

1. If both `gjelder_fra` and `gjelder_til` are before `arbeidssoeker_fra`: do not populate (`arbeidsledig_fra` stays `NULL`).
2. If `gjelder_fra` is before `arbeidssoeker_fra` and `gjelder_til` is after `arbeidssoeker_fra`: set `arbeidsledig_fra = arbeidssoeker_fra`.
3. If both `gjelder_fra` and `gjelder_til` are after `arbeidssoeker_fra`: set `arbeidsledig_fra = gjelder_fra`.

For subsequent bekreftelser where `arbeidsledig_fra` is already populated, keep existing behavior unchanged.

### 2) Periode processing derives from all bekreftelser for a period

In `PeriodeProcessor::utled_arbeidsledighet_fra_bekreftelser`:

- Bekreftelser are read ordered by `gjelder_fra`.
- Fold logic over the ordered rows:
  - `har_jobbet == true` => reset derived value to `NULL`
  - `har_jobbet == false` and value is currently `NULL` => apply the same initial-population guard as in event processing, using `periode.startet_tidspunkt`:
    1. If both `gjelder_fra` and `gjelder_til` are before `periode.startet_tidspunkt`: do not populate (`arbeidsledig_fra` stays `NULL`).
    2. If `gjelder_fra` is before `periode.startet_tidspunkt` and `gjelder_til` is after `periode.startet_tidspunkt`: set `arbeidsledig_fra = periode.startet_tidspunkt`.
    3. If both `gjelder_fra` and `gjelder_til` are after `periode.startet_tidspunkt`: set `arbeidsledig_fra = gjelder_fra`.
  - `har_jobbet == false` and value already set => keep existing value

Net effect is identical to event-by-event logic: the earliest `gjelder_fra` after the last `har_jobbet=true` becomes `arbeidsledig_fra`; if the latest relevant state indicates work, value ends up `NULL`.

### 3) Periode processing fallback to previous kartlegging

In `PeriodeProcessor::utled_arbeidsledighet_fra_tidligere_kartlegging`:

- If current period has derivable value from its own bekreftelser, that value wins.
- Otherwise, look up latest previous kartlegging for same `arbeidssoeker_id`.
- Carry over previous `arbeidsledig_fra` only when:
  - previous period has `arbeidsledig_fra` set,
  - previous period is closed (`arbeidssoeker_til` is set),
  - gap from previous `arbeidssoeker_til` to current period start is less than `periode_gap_grense_for_ledighet` (configured as `14` days).
- If any of those checks fail, return `NULL`.

### 4) Persisting the value

Both processors persist via `kartlegging::update` (or `insert` for new rows), writing `arbeidsledig_fra` directly to `kartlegginger.arbeidsledig_fra`.

## Gap: `BekreftelseProcessor` lacks the tidligere-kartlegging lookback

The recalculation is done by clearing `arbeidsledig_fra` and rewinding the HWM for
**only** the bekreftelse topic (`paw.arbeidssoker-bekreftelse-v1`), not the periode topic.
That means `BekreftelseProcessor` alone must be able to fully repopulate `arbeidsledig_fra`
during replay — no periode events will re-arrive to trigger `PeriodeProcessor`'s lookback.

Today, only `PeriodeProcessor::utled_arbeidsledighet_fra_tidligere_kartlegging` has the
fallback described in section 3 above (carry over the previous closed period's
`arbeidsledig_fra` when the gap is under `periode_gap_grense_for_ledighet`).
`BekreftelseProcessor::utled_arbeidsledighet_fra_bekreftelse` has no equivalent: it only ever
derives from the current period's own bekreftelser (the boundary rule from section 2) or
keeps the kartlegging row's existing value. If a bekreftelse's `gjelder_fra`/`gjelder_til` are
both before `arbeidssoeker_fra` (case 1 — "do not populate"), bekreftelse-only replay would
leave `arbeidsledig_fra = NULL` even in cases where the tidligere-kartlegging lookback should
have carried a value over when the period was first created — a regression versus current
production behavior.

**Fix:** extract the lookback into a shared module (e.g. `kartlegging_process`, alongside the
boundary rule already implemented there) so both processors use the same logic:

- A pure, unit-testable helper encoding the gap-check rule, e.g.
  `overfor_ledighet_fra_tidligere_periode(tidligere_arbeidsledig_fra: Option<DateTime<Utc>>, tidligere_arbeidssoeker_til: Option<DateTime<Utc>>, periode_startet: DateTime<Utc>, periode_gap_grense_dager: i64) -> Option<DateTime<Utc>>`.
  Rules: no previous value → `None`; previous period still open (`arbeidssoeker_til == None`)
  → `None`; gap in days `>=` the grense → `None`; otherwise → carry over
  `tidligere_arbeidsledig_fra`.
- An async wrapper in the same module, e.g.
  `utled_arbeidsledig_fra_fra_tidligere_kartlegging(tx, arbeidssoeker_id, periode_startet, periode_gap_grense_dager) -> anyhow::Result<Option<DateTime<Utc>>>`,
  performing the `kartlegging::select_latest_by_arbeidssoeker_id` lookup and delegating to the
  pure helper above.
- `PeriodeProcessor::utled_arbeidsledighet_fra_tidligere_kartlegging` becomes a thin wrapper:
  try `utled_arbeidsledighet_fra_bekreftelser` first, else call the shared fallback.
- `BekreftelseProcessor::utled_arbeidsledighet_fra_bekreftelse` becomes `async` (needs DB
  access) and, when the current-period boundary rule / existing value yields `None`, calls the
  same shared fallback using `kartlegging_row.arbeidssoeker_id` and
  `kartlegging_row.arbeidssoeker_fra` as `periode_startet`.
  - `BekreftelseProcessor` currently has no `AppConfig` (needed for
    `periode_gap_grense_for_ledighet`); add an `app_config: Arc<AppConfig>` field and thread it
    through `BekreftelseProcessor::new(...)`, updating both construction call sites
    (`MessageProcessor` wiring in `message_process.rs`, which already holds `app_config`, and
    the `TestContext` test harness in `bekreftelse_process.rs`).
  - Update the call site in `process_payload` to `.await` the now-async method and pass `tx`.

This preserves existing-value-preserved behavior (`Some(x) => Some(x)`) and only adds the
fallback when the current period truly has nothing of its own yet, matching what
`PeriodeProcessor` already does today for freshly-created periods.

## Plan

- Audit every write path that can set or clear `arbeidsledig_fra` (`bekreftelse_process` and `periode_process`) and align them with the new initial-population guard against `arbeidssoeker_fra`.
- Implement the new boundary-aware initial-population rule in both event-by-event (`BekreftelseProcessor`) and replay/derivation (`PeriodeProcessor`) paths, using the same `gjelder_fra`/`gjelder_til` checks against period start, while preserving existing behavior for already-populated values.
- Extract both the boundary rule and the tidligere-kartlegging lookback (previous-period carry-over) into a shared module (`kartlegging_process`), and make `BekreftelseProcessor` use the lookback too — required because replay only rewinds the bekreftelse topic, so `BekreftelseProcessor` must be able to fully repopulate `arbeidsledig_fra` on its own, including the previous-period fallback `PeriodeProcessor` already has.
- Add a new versioned Flyway migration that clears `arbeidsledig_fra` in `kartlegginger`.
- Extend the migration to reset HWM rows for `paw.arbeidssoker-bekreftelse-v1` to `0` for the current HWM version and all partitions in that topic.
- Keep the reset scoped to the bekreftelse topic only; do not rewind the other Kafka topics.
- Add regression coverage for all three initial-population scenarios (before/before, before/after overlap, after/after), for the extracted tidligere-kartlegging lookback (previous value unset, previous period still open, gap under/at/over the configured grense), and ensure replay path still restores `arbeidsledig_fra` correctly after reset — including the previous-period carry-over now handled by `BekreftelseProcessor`.

## Todo list

1. Auditing recalculation rules
2. Designing Flyway reset migration
3. Extracting shared boundary rule + tidligere-kartlegging lookback into `kartlegging_process`, and wiring `BekreftelseProcessor` (now async, with `AppConfig`) to use the lookback
4. Implementing data reset + HWM rewind
5. Adding regression coverage (boundary rules, lookback fallback in both processors, reset+replay)

## Notes

- HWM is runtime state, but in this case it is being manipulated through a versioned migration for the correction.
- Because Flyway migrations run once, each future recalculation should be added as a new versioned migration.
- The reset must be safe to run on already-correct rows and should not touch topics outside bekreftelse.
- Since only the bekreftelse topic HWM is rewound, `BekreftelseProcessor` must be able to fully reproduce what `PeriodeProcessor` would have derived, including the previous-period lookback — hence the extraction into a shared module rather than duplicating the logic.
