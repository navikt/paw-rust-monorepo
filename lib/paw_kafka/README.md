# paw_kafka::hwm — observability

Denne mappen eksponerer Prometheus-metrikker (counters, gauges, histogram) og
tracing-spans (med events) for high-water-mark Kafka-consumeren. Dette
dokumentet lister dem opp og viser eksempler på PromQL som kan brukes i
Grafana-dashboard.

## Counters

### `kafka_messages_processed_total`
*Kilde: `src/hwm/hwm_message_processor.rs`*

Antall Kafka-meldinger som er forsøkt prosessert av `MessageProcessor`.

Labels:
- `above_hwm` (`true`/`false`) — om meldingens offset var over lagret HWM
- `topic`
- `partition`
- `error` (`true`/`false`) — om `process_message` feilet

Grafana-eksempel — feilrate for prosesserte meldinger per topic:
```promql
sum by (topic) (rate(kafka_messages_processed_total{error="true"}[5m]))
/
sum by (topic) (rate(kafka_messages_processed_total{above_hwm="true"}[5m]))
```

### `paw_kafka_stream_messages_total`
*Kilde: `src/stream/stream_wrapper.rs`*

Meldinger levert av stream-wrapperen, fordelt på om tidsstempelet hoppet
bakover i den multipleksede streamen (på tvers av partisjoner) og om
meldingen selv hadde et monotont stigende tidsstempel innenfor sin egen
topic-partition. `source_in_sequence = false` betyr at bakoverhoppet skyldes
selve datakilden, ikke multipleksingen.

Labels:
- `back_in_time` (`true`/`false`)
- `source_in_sequence` (`true`/`false`) — om meldingens tidsstempel var
  monotont stigende innenfor sin topic-partition

Grafana-eksempel — andel meldinger med bakoverhopp i tid:
```promql
sum(rate(paw_kafka_stream_messages_total{back_in_time="true"}[5m]))
/
sum(rate(paw_kafka_stream_messages_total[5m]))
```

### `paw_kafka_stream_receive_total`
*Kilde: `src/stream/stream_wrapper.rs`*

Kall til `receive()` på streamen, fordelt på resultat.

Labels:
- `result` (`message` / `waiting` / `empty` / `error`)

Grafana-eksempel — andel `receive()`-kall som venter på stalled queues:
```promql
sum(rate(paw_kafka_stream_receive_total{result="waiting"}[5m]))
/
sum(rate(paw_kafka_stream_receive_total[5m]))
```

### `paw_kafka_stream_main_queue_messages_total`
*Kilde: `src/stream/stream_wrapper.rs`*

Meldinger hentet fra hoved-consumerens kø (før de fordeles til
partisjonskøene), fordelt på utfall.

Labels:
- `topic`
- `outcome` (`queued` / `not_assigned` / `below_hwm`)

Grafana-eksempel — meldinger droppet fordi de er under HWM, per topic:
```promql
sum by (topic) (rate(paw_kafka_stream_main_queue_messages_total{outcome="below_hwm"}[5m]))
```

## Histogram

### `paw_kafka_stream_back_in_time_ms`
*Kilde: `src/stream/stream_wrapper.rs`*

Størrelsen (i ms) på bakoverhopp i tidsstempel i den multipleksede streamen.
Eksponentielle bøtter (`exponential_buckets(1.0, 10.0, 9)`), altså 1 ms,
10 ms, 100 ms, ..., opp til 10^8 ms.

Labels:
- `topic`
- `source_in_sequence` (`true`/`false`) — samme betydning som på
  `paw_kafka_stream_messages_total`. `true` betyr at meldingens tidsstempel
  var monotont stigende innenfor sin topic-partition, så bakoverhoppet
  skyldes multipleksingen. `false` betyr at kilden selv hoppet bakover,
  eller at meldingen manglet tidsstempel.

Grafana-eksempel — p99 for størrelsen på bakoverhopp per topic, bare hopp
som skyldes multipleksingen:
```promql
histogram_quantile(
  0.99,
  sum by (le, topic) (rate(paw_kafka_stream_back_in_time_ms_bucket{source_in_sequence="true"}[5m]))
)
```

### `paw_kafka_stream_message_age_seconds`
*Kilde: `src/stream/stream_wrapper.rs`*

Alderen (i sekunder) på meldingen når `receive()` leverer den: nå minus
meldingens tidsstempel. Med `CreateTime` er tiden hos produsenten med, så
verdien er et øvre anslag på latensen fra broker til applikasjonen. Tiden
meldingen holdes igjen av `grace` er med. Tidsstempler fram i tid
(klokkeskjevhet) registreres som 0, og meldinger uten tidsstempel registreres
ikke. Eksponentielle bøtter (`exponential_buckets(0.005, 2.0, 20)`), altså
5 ms, 10 ms, 20 ms, ..., opp til omtrent 45 min.

Under replay havner meldingene i de øverste bøttene eller i `+Inf`, og
kvantiler over et vindu med replay blir misvisende.

Labels:
- `topic`

Grafana-eksempel — p99 for alder ved levering per topic:
```promql
histogram_quantile(
  0.99,
  sum by (le, topic) (rate(paw_kafka_stream_message_age_seconds_bucket[5m]))
)
```

Grafana-eksempel — andel meldinger levert innen 2,56 s (tåler replay bedre
enn kvantiler):
```promql
sum(rate(paw_kafka_stream_message_age_seconds_bucket{le="2.56"}[5m]))
/
sum(rate(paw_kafka_stream_message_age_seconds_count[5m]))
```

## Gauges

Alle fire under er per `(topic, partition)` og hører til
`src/stream/queue_handler.rs`. De fjernes (`remove_label_values`) når
partisjonen mistes ved rebalansering.

### `paw_kafka_stream_queue_handler_last_timestamp`
Tidsstempelet (ms) til meldingen som sist ble hentet ut av kø-handleren
(`take_head`). Beholder verdien fra forrige uttak selv om køen blir tom
etterpå — den er kun `NaN` før første melding er hentet ut.

### `paw_kafka_stream_queue_handler_next_timestamp`
Tidsstempelet (ms) til meldingen fremst i køen, altså den som leveres neste
gang. Settes til `NaN` når køen er tom (ingen neste melding å vise ennå).

### `paw_kafka_stream_queue_handler_depth`
Antall meldinger som for øyeblikket er bufret internt i kø-handleren for
denne partisjonen.

### `paw_kafka_stream_queue_handler_lag`
Totalt antall meldinger som ennå ikke er levert til applikasjonen for denne
partisjonen: ikke hentet fra broker (`hi_offset - next_offset`) pluss bufret i
rdkafkas interne kø (`fetchq_cnt`) pluss bufret i kø-handlerens eget
hode. `NaN` inntil første `stats()`-callback har satt offsets for partisjonen.

Grafana-eksempel — kø-dybde og lag for en spesifikk topic:
```promql
paw_kafka_stream_queue_handler_depth{topic="min-topic"}
paw_kafka_stream_queue_handler_lag{topic="min-topic"}
```

Grafana-eksempel — partisjoner som har stått stille (samme `next_timestamp`
gjennom hele intervallet) men fortsatt har lag, kan indikere en stalled queue:
```promql
paw_kafka_stream_queue_handler_lag > 0
and
changes(paw_kafka_stream_queue_handler_next_timestamp[5m]) == 0
```

## Spans (traces)

### `kafka_message_process`
*Kilde: `src/hwm/hwm_message_processor.rs`*

Opprettet med `tracing::info_span!` per melding som prosesseres, med
`otel.name` satt til `"{topic} process"`. Fjernkontekst (parent trace) hentes
fra Kafka-meldingens headers (`extract_remote_trace_context`) slik at
tracen kan kobles til produsentens trace i Grafana Tempo.

Felter:
- `messaging.system = "kafka"`
- `messaging.destination.name` (topic)
- `messaging.destination.partition.id`
- `messaging.kafka.message.offset`

Events i denne spanen: `tracing::trace!`/`tracing::error!` ved
suksess/feil i `process_message`.

### `paw_kafka_stream.receive`
*Kilde: `src/stream/stream_wrapper.rs`*

Wrapper rundt hvert kall til `receive()`.

Felter (satt med `Span::record`, tomme til de er kjent):
- `topic`, `partition`, `offset`, `timestamp` — enten meldingens egne verdier,
  eller `"waiting"`/`"none"` når ingen melding ble levert
- `back_in_time_ms` — satt kun når meldingens tidsstempel hoppet bakover

### `paw_kafka_stream.load`
*Kilde: `src/stream/stream_wrapper.rs`*

Spenner over `update()`-kallene mot alle partisjonskøer i én `receive()`.

Felt: `topic_partitions` (antall partisjoner som ble oppdatert).

### `paw_kafka_stream.queue_update`
*Kilde: `src/stream/queue_handler.rs`*

Én per partisjonskø sin `update()`.

Felt: `topic`, `partition`, `current_queue_size`.

Event i denne spanen: `tracing::warn!` med navn
`kafka.partition_timestamp_out_of_sequence` når en melding ankommer med
lavere tidsstempel enn forrige melding på samme partisjon (`push`).

### `paw_kafka_stream.get_rebalance_events`
*Kilde: `src/stream/stream_wrapper.rs`*

Spenner over uttrekk av ventende rebalanse-/statistikk-oppdateringer fra den
interne kanalen.

### `paw_kafka_stream.handle_rebalance_events`
*Kilde: `src/stream/stream_wrapper.rs`*

Spenner over håndtering av en batch med `TopicPartitionUpdate`-hendelser
(assign/revoke/hi-offset).

Felt: `topic_update_events` (antall hendelser i batchen).

Event i `track_stream_time` (kjøres inne i `paw_kafka_stream.receive`-spanen,
ikke i en egen span): `tracing::warn!` med navn `kafka.back_in_time` når
strømmens nyeste sette tidsstempel for en partisjon går bakover.

Grafana Tempo-eksempel — finn traces der en melding hang lenge i en
`paw_kafka_stream.queue_update`-span (treg henting fra broker):
```
{ name = "paw_kafka_stream.queue_update" && duration > 500ms }
```

Grafana Tempo-eksempel — finn traces med bakoverhopp i tid, ved å filtrere på
spanfeltet som er satt på `paw_kafka_stream.receive`:
```
{ name = "paw_kafka_stream.receive" && span.back_in_time_ms > 0 }
```
