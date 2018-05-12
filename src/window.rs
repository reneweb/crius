use std::collections::vec_deque::VecDeque;
use std::time::{Instant, Duration};
use command::Config;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Point {
    SUCCESS,
    FAILURE,
}

#[derive(Clone, Debug)]
struct Bucket {
    points: Vec<Point>,
    timestamp: Instant,
}

impl Bucket {
    fn new() -> Bucket {
        Bucket {
            points: Vec::new(),
            timestamp: Instant::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Window {
    buckets: VecDeque<Bucket>,
    bucket_ms: Duration,
    buckets_nr: i32,
}

impl Window {
    pub fn new(config: Config) -> Self {
        Window {
            buckets: VecDeque::new(),
            bucket_ms: Duration::from_millis(config.bucket_size_in_ms),
            buckets_nr: config.buckets_in_window,
        }
    }

    pub fn add_point(&mut self, point: Point) {
        let current_bucket = self.update_window_returning_latest_bucket();
        current_bucket.points.push(point);
    }

    pub fn clear_window(&mut self) {
        self.buckets.clear();
    }

    pub fn update_and_get_points(&mut self) -> Vec<Point> {
        self.update_window_returning_latest_bucket();
        self.buckets.iter().fold(vec![], |mut acc, bucket| {
            acc.extend(&bucket.points);
            return acc;
        })
    }

    fn update_window_returning_latest_bucket(&mut self) -> &mut Bucket {
        let now = Instant::now();
        let latest_threshold = self.buckets.back()
            .map(|bucket| bucket.timestamp + self.bucket_ms);

        if let Some(threshold) = latest_threshold {
            // Return the latest bucket if it is still current:
            if threshold > now {
                return self.buckets.back_mut().unwrap();
            }

            // Otherwise create and return a new bucket:
            let new_bucket = Bucket {
                points: Vec::new(),
                timestamp: threshold,
            };

            self.buckets.push_back(new_bucket);
            if self.buckets.len() > self.buckets_nr as usize {
                self.buckets.pop_front();
            }
            return self.buckets.back_mut().unwrap();
        } else {
            // Create a bucket if there aren't any in the window currently:
            let first_bucket = Bucket::new();
            self.buckets.push_back(first_bucket);
            return self.buckets.back_mut().unwrap();
        }
    }
}
