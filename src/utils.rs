use crate::prelude::*;

pub fn format_date(date: DateTime) -> String {
  date.format("%d.%m.%Y %H:%M").to_string()
}

pub fn format_duration(duration: TimeDelta) -> String {
  format!(
    "{}d {}h {}m",
    duration.num_days(),
    duration.num_hours() % 24,
    duration.num_minutes() % 60
  )
}
