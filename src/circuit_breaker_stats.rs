use window::Window;
use window::Point;

#[derive(Clone, Debug)]
pub struct CircuitBreakerStats {
    pub window: Window,
}

impl CircuitBreakerStats {
    pub fn add_point(&mut self, point: Point) {
        self.window.add_point(point)
    }

    pub fn clear(&mut self) {
        self.window.clear_window()
    }

    pub fn success_percentage(&mut self) -> i32 {
        let points = self.window.update_and_get_points();
        let success_nr = self.success_nr();
        if success_nr == 0 {
            return 0;
        } else {
            return (success_nr / points.len() as i32) * 100;
        }
    }

    pub fn error_percentage(&mut self) -> i32 {
        let points = self.window.update_and_get_points();
        let error_nr = self.error_nr();

        if error_nr == 0 {
            return 0;
        } else {
            return (error_nr / points.len() as i32) * 100;
        }
    }

    pub fn success_nr(&mut self) -> i32 {
        let points = self.window.update_and_get_points();
        let success_count = points
            .iter()
            .filter(|&&point| return point == Point::SUCCESS)
            .collect::<Vec<_>>()
            .len();

        success_count as i32
    }

    pub fn error_nr(&mut self) -> i32 {
        let points = self.window.update_and_get_points();
        let error_count = points
            .iter()
            .filter(|&&point| return point == Point::FAILURE)
            .collect::<Vec<_>>()
            .len();

        error_count as i32
    }
}
