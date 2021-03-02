DROP TABLE IF EXISTS "public".members;
-- ************************************** "public".members
CREATE TABLE IF NOT EXISTS "public".members (
  id bigint NOT NULL,
  guild_id bigint NOT NULL,
  coins bigint NOT NULL,
  last_daily timestamptz NOT NULL,
  CONSTRAINT member_of FOREIGN KEY (guild_id) REFERENCES "public".guilds ("id")
);
CREATE INDEX guild_members ON "public".members (guild_id);