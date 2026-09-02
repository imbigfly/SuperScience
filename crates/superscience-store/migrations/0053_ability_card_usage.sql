CREATE TABLE IF NOT EXISTS frame_ability_cards (
  frame_id   TEXT PRIMARY KEY REFERENCES frames(id) ON DELETE CASCADE,
  card_id    TEXT NOT NULL,
  card_name  TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ability_card_daily_resume (
  frame_id    TEXT NOT NULL,
  usage_date  TEXT NOT NULL,
  reported_at INTEGER NOT NULL,
  PRIMARY KEY (frame_id, usage_date)
);
