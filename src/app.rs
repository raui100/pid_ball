use web_time::{Duration, Instant};

use crate::sim::{Message, Simulation};
use eframe::egui;
use egui::{Color32, DragValue, Pos2, Vec2};
use egui_plot::{Corner, HLine, Legend, Line, Plot, PlotPoints};

use crate::default::*;

struct Time {
    /// Point in time when the GUI has started
    gui: Instant,
    /// Duration of time that has elapsed in the simulation
    sim: Duration,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            gui: Instant::now(),
            sim: Duration::ZERO,
        }
    }
}

impl Time {
    /// Calculates the number of steps the simulation should step forward
    fn step(&mut self, sampling_time: Duration) -> u32 {
        let sim_dt = self.gui.elapsed() - self.sim; // Delta of real time and GUI time
        let steps = sim_dt.as_secs_f32().div_euclid(sampling_time.as_secs_f32()); // Whole number
        self.sim += sampling_time.mul_f32(steps);

        steps as u32
    }
}

struct Input {
    kp: Cache<f32>,
    ki: Cache<f32>,
    kd: Cache<f32>,
    target: Cache<f32>,
    sampling_rate: Cache<u32>,
    noise: Cache<f32>,
    gravitation: Cache<f32>,
    max_force: Cache<f32>,
    max_force_rate: Cache<f32>,
    hold_ball: Cache<bool>,
}

struct Cache<T: PartialEq + Clone> {
    pub val: T,
    prev: T,
}

impl<T: PartialEq + Clone> Cache<T> {
    fn new(val: T) -> Self {
        Self {
            val: val.clone(),
            prev: val,
        }
    }

    fn changed(&mut self) -> Option<T> {
        if self.val != self.prev {
            self.prev = self.val.clone();
            Some(self.val.clone())
        } else {
            None
        }
    }

    fn get_mut(&mut self) -> &mut T {
        &mut self.val
    }

    fn get(&self) -> T {
        self.val.clone()
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            kp: Cache::new(KP),
            ki: Cache::new(KI),
            kd: Cache::new(KD),
            target: Cache::new(TARGET),
            sampling_rate: Cache::new(SAMPLING_RATE),
            noise: Cache::new(NOISE),
            gravitation: Cache::new(GRAVITATION),
            max_force: Cache::new(MAX_FORCE),
            max_force_rate: Cache::new(MAX_FORCE_RATE),
            hold_ball: Cache::new(HOLD_BALL),
        }
    }
}

impl Input {
    fn update(&mut self, sim: &mut Simulation) {
        // PID constants
        if let Some(val) = self.kp.changed() {
            sim.config(Message::Kp(val));
        }
        if let Some(val) = self.ki.changed() {
            sim.config(Message::Ki(val));
        }
        if let Some(val) = self.kd.changed() {
            sim.config(Message::Kd(val));
        }

        // PID Target
        if let Some(val) = self.target.changed() {
            sim.config(Message::Target(val));
        }

        // Sensor noise
        if let Some(val) = self.noise.changed() {
            sim.config(Message::Noise(val));
        }

        // Gravitation
        if let Some(val) = self.gravitation.changed() {
            sim.config(Message::Gravitation(val));
        }
        // Max. force
        if let Some(val) = self.max_force.changed() {
            sim.config(Message::MaxForce(val));
        }
        // Max. force rate
        if let Some(val) = self.max_force_rate.changed() {
            sim.config(Message::MaxForceRate(val));
        }
        // Hold ball
        if let Some(val) = self.hold_ball.changed() {
            sim.config(Message::HoldBall(val));
        }
    }
}

pub struct MyApp {
    input: Input,
    sim: Simulation,
    pos: Vec<f32>,
    vel: Vec<f32>,
    target: Vec<f32>,
    force: Vec<f32>,
    seconds: Vec<f32>,
    time: Time,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            input: Default::default(),
            sim: Default::default(),
            pos: Default::default(),
            vel: Default::default(),
            target: Default::default(),
            time: Default::default(),
            force: Default::default(),
            seconds: Default::default(),
        }
    }
}

impl MyApp {
    /// Clears every buffer
    fn clear(&mut self) {
        self.pos.clear();
        self.vel.clear();
        self.target.clear();
        self.force.clear();
        self.seconds.clear();
    }
    /// Restarts everything and discards user input
    fn reset(&mut self) {
        self.clear();
        self.sim.config(Message::Restart);  // restart simulation
        self.time = Default::default();
        self.input = Default::default();
    }
    /// Restarts everything but keeps user input
    fn restart(&mut self) {
        self.clear();
        self.sim.config(Message::Reset);  // resets simulation
        self.time = Default::default();
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint(); // Maximizing FPS

        // Updating the simulation config
        self.input.update(&mut self.sim);

        // Stepping the simulation forward
        let sampling_time = 1.0 / self.input.sampling_rate.get() as f32;
        let sampling_time = Duration::from_secs_f32(sampling_time);
        let steps = self.time.step(sampling_time);
        if ctx.frame_nr() > 10 {
            // GUI is stuttering for the first few samples
            let data = self.sim.step(steps, sampling_time);
            self.pos.push(data.pos);
            self.target.push(self.input.target.get());
            self.vel.push(data.vel);
            self.force.push(data.force);
            self.seconds.push(self.time.gui.elapsed().as_secs_f32());
        }

        egui::TopBottomPanel::top("config1").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Reset").on_hover_text("Clears plots and resets with default values").clicked() {
                    self.reset()
                }
                if ui.button("Restart").on_hover_text("Clears plots and resets with given values").clicked() {
                    self.restart();
                }
                ui.separator();
                ui.label("Noise [σ]");
                ui.add(
                    DragValue::new(self.input.noise.get_mut())
                        .speed(0.001)
                        .clamp_range(0.0..=1.0),
                );
                ui.separator();
                ui.label("Target");
                ui.add(
                    DragValue::new(self.input.target.get_mut())
                        .speed(0.01)
                        .clamp_range(0.25..=0.75),
                );
                ui.separator();
                ui.label("Sampling Rate [Hz]");
                ui.add(
                    DragValue::new(self.input.sampling_rate.get_mut())
                        .speed(0.1)
                        .clamp_range(1..=u32::MAX),
                );
                ui.separator();
                ui.label("P");
                ui.add(DragValue::new(self.input.kp.get_mut()).speed(1));
                ui.separator();
                ui.label("I");
                ui.add(DragValue::new(self.input.ki.get_mut()).speed(0.01));
                ui.separator();
                ui.label("D");
                ui.add(DragValue::new(self.input.kd.get_mut()).speed(0.1));
                
                // Link to egui
                ui.separator();
                ui.hyperlink_to("Source", "https://github.com/raui100/pid_ball");
                ui.hyperlink_to("Made with egui", "https://github.com/emilk/egui");
            });
        });
        egui::TopBottomPanel::top("config2").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Hold/Drop ball
                if self.input.hold_ball.get() {
                    if ui.button("Drop").on_hover_text("Holds the ball still").clicked() {
                        self.input.hold_ball.val = false;
                    }
                } else {
                    if ui.button("Hold").on_hover_text("Free floating ball").clicked() {
                        self.input.hold_ball.val = true;
                    }
                }
                ui.separator();

                // Gravitation
                ui.label("Gravitation")
                    .on_hover_text("Use '-9.81' for earth-like gravitation");
                ui.add(DragValue::new(self.input.gravitation.get_mut()).speed(0.1));
                ui.separator();
                
                // Max force
                ui.label("Max. force [N]")
                    .on_hover_text("The ball weighs 1 Kg");
                ui.add(
                    DragValue::new(self.input.max_force.get_mut())
                        .speed(1.0)
                        .clamp_range(0.0..=f32::INFINITY),
                );
                ui.separator();

                // Max force rate
                ui.label("Max. force rate [N/s]")
                    .on_hover_text("Limits the rate with which the force can adapt");
                ui.add(
                    DragValue::new(self.input.max_force_rate.get_mut())
                        .speed(0.1)
                        .clamp_range(0.0..=f32::INFINITY),
                );
            });
        });

        // Painting the ball
        let y_width = ctx.available_rect().width();
        egui::SidePanel::left("ball")
            .default_width(y_width * 0.2)  // Gives 20% of the space to the animation
            .show(ctx, |ui| {
                if let Some(&pos) = self.pos.last() {
                    let Vec2 { x, y } = ui.available_size();
                    let radius = x * 0.8 * 0.5; // Taking 80% of the available space
                    let y_ball = y - pos * y;
                    let x_ball = x * 0.56;
                    ui.painter()
                        .circle_filled(Pos2::new(x_ball, y_ball), radius, Color32::RED);
                }
            });

        // Plotting position and velocity of the ball
        egui::CentralPanel::default().show(ctx, |ui| {
            let height = ui.available_height() / 3.0;
            let group_id = ui.id().with("x_axis");
            let line = |y: &[f32]| {
                PlotPoints::from_iter(
                    self.seconds
                        .iter()
                        .zip(y)
                        .map(|(x, y)| [*x as f64, *y as f64]),
                )
            };

            let legend = Legend {
                text_style: egui::TextStyle::Heading,
                background_alpha: 1.0,
                position: Corner::LeftBottom,
            };

            // Position
            Plot::new("pos")
                .legend(legend.clone())
                .link_axis(group_id, true, false)
                .show_axes([false, true])
                .height(height)
                .show(ui, |ui| {
                    // Plotting the current target as horizontal line
                    ui.hline(HLine::new(self.input.target.val).color(Color32::BLACK));
                    // Plotting the target over time
                    ui.line(
                        Line::new(line(&self.target))
                            .name("Target [m]")
                            .color(Color32::GRAY),
                    );
                    // Plotting the position of the ball
                    ui.line(
                        Line::new(line(&self.pos))
                            .name("Position [m]")
                            .highlight(true)
                            .color(Color32::RED),
                    );
                });

            // Velocity
            Plot::new("vel")
                .link_axis(group_id, true, false)
                .show_axes([false, true])
                .legend(legend.clone())
                .height(height)
                .show(ui, |ui| {
                    // Plotting the velocity
                    ui.line(
                        Line::new(line(&self.vel))
                            .name("Velocity [m/s]")
                            .highlight(true)
                            .color(Color32::BLUE),
                    );
                });

            // Force
            Plot::new("force")
                .link_axis(group_id, true, false)
                .legend(legend)
                .height(height)
                .x_axis_label("Time [s]")
                .show(ui, |ui| {
                    // Plotting the velocity
                    ui.line(
                        Line::new(line(&self.force))
                            .name("Force [N]")
                            .highlight(true)
                            .color(Color32::GREEN),
                    );
                });
        });
    }
}
