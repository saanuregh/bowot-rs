DROP TABLE IF EXISTS "public".hydrate_reminders;
-- ************************************** "public".hydrate_reminders
CREATE TABLE IF NOT EXISTS "public".hydrate_reminders (
  "id" bigint NOT NULL,
  CONSTRAINT PK_hydrate_reminders PRIMARY KEY ("id")
);