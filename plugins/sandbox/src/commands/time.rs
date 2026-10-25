use ferrosa_wit::{clock_time, server};

fn parse_clock(arg: &str) -> Option<(i32, i32)> {
    let (hours, minutes) = arg.split_once(':').unwrap_or((arg, "0"));
    let (h, m) = (hours.parse::<i32>().ok()?, minutes.parse::<i32>().ok()?);

    ((0..24).contains(&h) && (0..60).contains(&m)).then_some((h, m))
}

pub fn set(args: &[&str]) {
    if let Some(arg) = args.first() {
        match parse_clock(arg) {
            Some((h, m)) => server::set_world_time_clock(h, m),
            None => {
                server::announce("Usage: /time <HH:MM>");
                return;
            }
        }
    }

    let now = server::world_time();

    server::announce(&format!("Time is {:02}:{:02}", now / clock_time(1, 0), now / clock_time(0, 1) % 60));
}
