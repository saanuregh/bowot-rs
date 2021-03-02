DROP TABLE IF EXISTS "public".guilds;
-- ************************************** "public".guilds
CREATE TABLE IF NOT EXISTS "public".guilds (
  "id" bigint NOT NULL,
  prefix varchar(10) NOT NULL,
  disabled_commands text [] NOT NULL,
  CONSTRAINT PK_guilds PRIMARY KEY ("id")
);