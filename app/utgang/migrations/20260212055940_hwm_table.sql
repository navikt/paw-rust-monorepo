CREATE TABLE hwm
(
    version   SMALLINT     NOT NULL,
    topic     VARCHAR(255) NOT NULL,
    partition SMALLINT     NOT NULL,
    hwm       BIGINT       NOT NULL,
    PRIMARY KEY (version, topic, partition)
);
-- Add migration script here
