DROP TABLE IF EXISTS "public".triggers;
-- ************************************** "public".triggers
CREATE TABLE IF NOT EXISTS "public".triggers (
  phrase text NOT NULL,
  reply text NOT NULL,
  guild_id bigint NOT NULL,
  CONSTRAINT PK_triggers PRIMARY KEY (phrase),
  CONSTRAINT trigger_of FOREIGN KEY (guild_id) REFERENCES "public".guilds ("id")
);
CREATE INDEX guild_triggers ON "public".triggers (guild_id);