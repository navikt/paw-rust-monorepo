INSERT INTO kartlegginger (parent_id,
                         periode_id,
                         arbeidssoeker_siden,
                         arbeidsledig_siden)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '24849098329'),
        'db8270e6-ea64-469f-93b4-69fb07a472c2',
        '2026-04-30 08:02:54',
        '2026-04-30 08:02:54');

INSERT INTO kartlegginger (parent_id,
                         periode_id,
                         arbeidssoeker_siden,
                         arbeidsledig_siden)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '16488315440'),
        'd2151e5d-d3a2-40b1-9a9a-6ae89a85e065',
        '2026-06-03 08:17:43',
        '2026-06-03 08:17:43');

INSERT INTO kartlegginger (parent_id,
                         periode_id,
                         arbeidssoeker_siden,
                         arbeidsledig_siden)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '09448718961'),
        'd379a0c3-39e8-4c14-ab3e-2328a3fe80cb',
        '2026-06-04 12:59:36',
        '2026-06-04 12:59:36');
