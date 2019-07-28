use quicksilver::{
    geom::{Circle, Rectangle, Transform, Triangle, Vector},
    graphics::{Background, Color},
    input::Key,
    lifecycle::{run, Settings, State, Window},
    Result,
};
use rand::prelude::*;

const SCREEN_WIDTH: f32 = 800.;
const SCREEN_HEIGHT: f32 = 600.;
const SKI_WIDTH: f32 = 10.;
const SKI_LENGTH: f32 = 50.;
const SKI_TIP_LEN: f32 = 20.;
const N_DOORS_IN_SCREEN: usize = 3;
const DOOR_POLE_RADIUS: f32 = 4.;
const DOOR_WIDTH: f32 = 150.;
const STEERING_SPEED: f32 = 3.5;
const MAX_ANGLE: f32 = 75.;
const SKI_MARGIN: f32 = 12.;
const MIN_TIME_DURATION: f64 = 0.1;
const ALONG_ACCELERATION: f32 = 0.06;
const DRAG_FACTOR: f32 = 0.02;
const TOTAL_N_DOORS: usize = 8;

#[derive(Debug)]
enum Mode {
    Ready,
    Running,
    Finished,
    Failed,
}

struct Screen {
    doors: Vec<(f32, f32)>,
    ski_across_offset: f32,
    direction: f32,
    forward_speed: f32,
    doors_along_offset: f32,
    elapsed_sec: f64,
    elapsed_shown_sec: f64,
    mode: Mode,
    entered_door: bool,
    disappeared_doors: usize,
}

impl Screen {
    fn get_random_door(door_at_right: bool) -> (f32, f32) {
        let mut rng = thread_rng();
        let pole_pos = rng.gen_range(-DOOR_WIDTH / 2., SCREEN_WIDTH / 2. - DOOR_WIDTH * 1.5);
        if door_at_right {
            (pole_pos, pole_pos + DOOR_WIDTH)
        } else {
            (-pole_pos - DOOR_WIDTH, -pole_pos)
        }
    }

    fn steer(&mut self, direction: f32) {
        self.direction += STEERING_SPEED * direction;
        if self.direction > MAX_ANGLE {
            self.direction = MAX_ANGLE;
        } else if self.direction < -MAX_ANGLE {
            self.direction = -MAX_ANGLE;
        }
    }
}

fn deg_to_rad(angle: f32) -> f32 {
    angle / 180. * std::f32::consts::PI
}

// Assume the following dynamics
// * there is a positive acceleration that is proportional
//   to the along component of direction
// * there is a negative acceleration (deceleration)
//   that is proportional to the velocity

impl State for Screen {
    fn new() -> Result<Screen> {
        let mut doors = Vec::new();
        for i in 0..TOTAL_N_DOORS {
            doors.push(Self::get_random_door(i % 2 == 0));
        }
        Ok(Screen {
            doors,
            ski_across_offset: 0.,
            direction: 0.,
            forward_speed: 0.,
            doors_along_offset: 0.,
            elapsed_sec: 0.,
            elapsed_shown_sec: 0.,
            mode: Mode::Ready,
            entered_door: false,
            disappeared_doors: 0,
        })
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        match self.mode {
            Mode::Ready => {
                if window.keyboard()[Key::Space].is_down() {
                    self.mode = Mode::Running;
                }
            }
            Mode::Running => {
                let angle = deg_to_rad(self.direction);
                self.forward_speed +=
                    ALONG_ACCELERATION * angle.cos() - DRAG_FACTOR * self.forward_speed;
                let along_speed = self.forward_speed * angle.cos();
                self.ski_across_offset += self.forward_speed * angle.sin();
                if self.ski_across_offset < -SCREEN_WIDTH / 2. + SKI_MARGIN {
                    self.ski_across_offset = -SCREEN_WIDTH / 2. + SKI_MARGIN;
                }
                if self.ski_across_offset > SCREEN_WIDTH / 2. - SKI_MARGIN {
                    self.ski_across_offset = SCREEN_WIDTH / 2. - SKI_MARGIN;
                }
                self.doors_along_offset += along_speed;
                let max_doors_along_offset = SCREEN_HEIGHT / N_DOORS_IN_SCREEN as f32;
                if self.doors_along_offset > max_doors_along_offset {
                    self.doors_along_offset -= max_doors_along_offset;
                    self.disappeared_doors += 1;
                }
                self.elapsed_sec += window.update_rate() / 1000.;

                if self.elapsed_sec - self.elapsed_shown_sec >= MIN_TIME_DURATION {
                    self.elapsed_shown_sec = self.elapsed_sec;
                }

                // If the ski tip is over a door, and before it wasn't,
                // check whether it is within the door.
                let ski_tip_along = SCREEN_HEIGHT * 15. / 16. - SKI_LENGTH / 2. - SKI_TIP_LEN;

                let ski_tip_across = SCREEN_WIDTH / 2. + self.ski_across_offset;

                let n_next_door = self.disappeared_doors;
                let next_door = &self.doors[n_next_door];
                let left_pole_offset = SCREEN_WIDTH / 2. + next_door.0 + DOOR_POLE_RADIUS;
                let right_pole_offset = SCREEN_WIDTH / 2. + next_door.1 - DOOR_POLE_RADIUS;
                let next_door_along = self.doors_along_offset + SCREEN_HEIGHT
                    - SCREEN_HEIGHT / N_DOORS_IN_SCREEN as f32;
                if ski_tip_along <= next_door_along {
                    if !self.entered_door {
                        if ski_tip_across < left_pole_offset || ski_tip_across > right_pole_offset {
                            self.mode = Mode::Failed;
                        } else if self.disappeared_doors == TOTAL_N_DOORS - 1 {
                            self.mode = Mode::Finished;
                        }
                        self.entered_door = true;
                    }
                } else {
                    self.entered_door = false;
                }
            }
            Mode::Failed | Mode::Finished => {
                if window.keyboard()[Key::R].is_down() {
                    *self = Screen::new().unwrap();
                }
            }
        }
        if window.keyboard()[Key::Right].is_down() {
            self.steer(1.);
        }
        if window.keyboard()[Key::Left].is_down() {
            self.steer(-1.);
        }
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::WHITE)?;
        for i_door in self.disappeared_doors..self.disappeared_doors + N_DOORS_IN_SCREEN {
            if i_door >= TOTAL_N_DOORS {
                break;
            }
            let door = self.doors[i_door];
            let pole_color = Background::Col(if i_door == TOTAL_N_DOORS - 1 {
                Color::GREEN
            } else {
                Color::BLUE
            });
            let doors_along_pos = self.doors_along_offset
                + SCREEN_HEIGHT / N_DOORS_IN_SCREEN as f32
                    * (self.disappeared_doors + N_DOORS_IN_SCREEN - 1 - i_door) as f32;
            window.draw(
                &Circle::new(
                    (SCREEN_WIDTH / 2. + door.0, doors_along_pos),
                    DOOR_POLE_RADIUS,
                ),
                pole_color,
            );
            window.draw(
                &Circle::new(
                    (SCREEN_WIDTH / 2. + door.1, doors_along_pos),
                    DOOR_POLE_RADIUS,
                ),
                pole_color,
            );
        }
        window.draw_ex(
            &Rectangle::new(
                (
                    SCREEN_WIDTH / 2. + self.ski_across_offset - SKI_WIDTH / 2.,
                    SCREEN_HEIGHT * 15. / 16. - SKI_LENGTH / 2.,
                ),
                (SKI_WIDTH, SKI_LENGTH),
            ),
            Background::Col(Color::PURPLE),
            Transform::translate(Vector::new(0, -SKI_LENGTH / 2. - SKI_TIP_LEN))
                * Transform::rotate(self.direction)
                * Transform::translate(Vector::new(0, SKI_LENGTH / 2. + SKI_TIP_LEN)),
            0,
        );

        window.draw_ex(
            &Triangle::new(
                Vector::new(
                    SCREEN_WIDTH / 2. + self.ski_across_offset - SKI_WIDTH / 2.,
                    SCREEN_HEIGHT * 15. / 16. - SKI_LENGTH / 2.,
                ),
                Vector::new(
                    SCREEN_WIDTH / 2. + self.ski_across_offset + SKI_WIDTH / 2.,
                    SCREEN_HEIGHT * 15. / 16. - SKI_LENGTH / 2.,
                ),
                Vector::new(
                    SCREEN_WIDTH / 2. + self.ski_across_offset,
                    SCREEN_HEIGHT * 15. / 16. - SKI_LENGTH / 2. - SKI_TIP_LEN,
                ),
            ),
            Background::Col(Color::INDIGO),
            Transform::translate(Vector::new(0, -SKI_TIP_LEN * 2. / 3.))
                * Transform::rotate(self.direction)
                * Transform::translate(Vector::new(0, SKI_TIP_LEN * 2. / 3.)),
            0,
        );

        Ok(())
    }
}

fn main() {
    run::<Screen>(
        "Slalom",
        Vector::new(SCREEN_WIDTH, SCREEN_HEIGHT),
        Settings {
            draw_rate: 40.,
            update_rate: 40.,
            ..Settings::default()
        },
    );
}