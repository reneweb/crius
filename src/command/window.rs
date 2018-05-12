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
        return Bucket {
            points: Vec::new(),
            timestamp: Instant::now(),
        };
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
        return Window {
            buckets: VecDeque::new(),
            bucket_ms: Duration::from_millis(config.bucket_size_in_ms.unwrap()),
            buckets_nr: config.buckets_in_window.unwrap(),
        };
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
        let points = self.buckets.iter().fold(vec![], |mut acc, bucket| {
            acc.extend(&bucket.points);
            return acc;
        });
        return points;
    }

    fn update_window_returning_latest_bucket(&mut self) -> &mut Bucket {
        let now = Instant::now();

        let has_buckets = self.buckets.back_mut().is_some();
        if !has_buckets {
            let first_bucket = Bucket::new();
            self.buckets.push_back(first_bucket);
            return self.buckets.back_mut().unwrap();
        } else {
            let latest_bucket_timestamp = self.get_latest_bucket().unwrap().timestamp;
            loop {
                if latest_bucket_timestamp + self.bucket_ms > now {
                    return self.get_latest_bucket().unwrap();
                } else {
                    let new_bucket = Bucket {
                        points: Vec::new(),
                        timestamp: latest_bucket_timestamp + self.bucket_ms,
                    };
                    self.buckets.push_back(new_bucket);
                    if self.buckets.len() > self.buckets_nr as usize {
                        self.buckets.pop_front();
                    }

                    return self.buckets.back_mut().unwrap();
                }
            }
        }
    }

    fn get_latest_bucket(&mut self) -> Option<&mut Bucket> {
        return self.buckets.back_mut();
    }
}
