INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '24849098329'),
        '4154',
        'Nasjonal oppfølgingsenhet',
        'ARBEIDSOPPFOLGING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '24849098329'),
        '0617',
        'Nav Hallingdal',
        'GEOGRAFISK_TILKNYTNING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '16488315440'),
        '0617',
        'Nav Hallingdal',
        'ARBEIDSOPPFOLGING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '16488315440'),
        '0617',
        'Nav Hallingdal',
        'GEOGRAFISK_TILKNYTNING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '09448718961'),
        '0617',
        'Nav Hallingdal',
        'GEOGRAFISK_TILKNYTNING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '24488623539'),
        '3030',
        'Nav Sogn',
        'GEOGRAFISK_TILKNYTNING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '02838698800'),
        '0118',
        'Nav Fredrikstad',
        'GEOGRAFISK_TILKNYTNING');

INSERT INTO tilknyttet_kontor (parent_id,
                               kontor_id,
                               kontor_navn,
                               kontor_type)
VALUES ((SELECT id FROM arbeidssoekere WHERE identitetsnummer = '28888896696'),
        '0604',
        'Nav Kongsberg',
        'GEOGRAFISK_TILKNYTNING');