CREATE TABLE hwm
(
    version          SMALLINT     NOT NULL,
    topic            VARCHAR(255) NOT NULL,
    partition        INT          NOT NULL,
    hwm              BIGINT       NOT NULL,
    status           VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
    status_timestamp TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (version, topic, partition)
);
