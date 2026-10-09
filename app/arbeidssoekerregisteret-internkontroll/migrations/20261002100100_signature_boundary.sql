CREATE TABLE signature_boundary (
    topic           VARCHAR(255) NOT NULL,
    partition       INTEGER NOT NULL,
    boundary_offset BIGINT NOT NULL,
    PRIMARY KEY (topic, partition),
    FOREIGN KEY (topic, partition, boundary_offset)
        REFERENCES kafka_record (topic, partition, offset)
);

