use crate::default::*;
use rand::rngs::ThreadRng;
use rand_distr::{Distribution, Normal};
use web_time::Duration;

/// Simulation of the floating ball
pub struct Simulation {
    pid: Pid,
    ball: Ball,
    ind: Inductor,
    sensor: Sensor,
    gravitation: f32,
    hold_ball: bool,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            pid: Default::default(),
            ball: Default::default(),
            ind: Default::default(),
            sensor: Default::default(),
            gravitation: GRAVITATION,
            hold_ball: HOLD_BALL,
        }
    }
}

impl Simulation {
    pub fn config(&mut self, msg: Message) {
        match msg {
            Message::Kp(kp) => self.pid.kp = kp,
            Message::Ki(ki) => self.pid.ki = ki,
            Message::Kd(kd) => self.pid.kd = kd,
            Message::Target(t) => self.pid.target = t,
            Message::Noise(s) => self.sensor.set_sigma(s),
            Message::Gravitation(g) => self.gravitation = g,
            Message::MaxForce(f) => self.ind.max_force = f,
            Message::MaxForceRate(f) => self.ind.max_force_rate = f,
            Message::HoldBall(b) => self.hold_ball = b,
            Message::Restart => *self = Default::default(),
            Message::Reset => self.reset(),
        }
    }
    pub fn reset(&mut self) {
        self.pid.reset();
        self.ball.reset();
        self.ind.reset();
        self.sensor.reset()
    }

    pub fn step(&mut self, steps: u32, sampling_time: Duration) -> Data {
        for _ in 0..steps {
            // Moving the ball
            if !self.hold_ball {
                let dis = (self.ball.pos - self.ind.pos).abs();
                let force = self.ind.force();
                let force = force / (1.0 + dis.powi(2));
                let force = force + self.gravitation;
                self.ball.step(force, sampling_time);
            }

            // Measuring the position of the ball
            let pos = self.sensor.pos(&self.ball);

            // Adapting the current on the induction
            self.pid.update(pos, sampling_time);
            let force = self.pid.total();
            self.ind.set_force(force, sampling_time);
        }
        Data {
            pos: self.ball.pos,
            vel: self.ball.vel,
            force: self.ind.force(),
        }
    }
}

pub enum Message {
    Kp(f32),
    Ki(f32),
    Kd(f32),
    Reset,
    Target(f32),
    Noise(f32),
    Gravitation(f32),
    MaxForce(f32),
    MaxForceRate(f32),
    HoldBall(bool),
    Restart,
}

pub struct Data {
    pub pos: f32,
    pub vel: f32,
    pub force: f32,
}

#[derive(Debug)]
pub struct Ball {
    pos: f32,
    vel: f32,
}

impl Default for Ball {
    fn default() -> Self {
        Self {
            pos: BALL_POS,
            vel: BALL_VEL,
        }
    }
}

impl Ball {
    pub fn reset(&mut self) {
        *self = Default::default();
    }
    pub fn step(&mut self, force: f32, delta_time: Duration) {
        let dt = delta_time.as_secs_f32();
        let delta_vel = 0.5 * dt * force;
        self.vel += delta_vel;
        self.pos += self.vel * dt;
        self.vel += delta_vel;
    }
}

pub struct Inductor {
    pos: f32,
    force: f32,
    max_force: f32,
    max_force_rate: f32,
}

impl Default for Inductor {
    fn default() -> Self {
        Self {
            pos: IND_POS,
            force: 0.0,
            max_force: MAX_FORCE,
            max_force_rate: MAX_FORCE_RATE,
        }
    }
}

impl Inductor {
    pub fn reset(&mut self) {
        self.force = 0.0;
    }

    fn force(&self) -> f32 {
        self.force
    }

    fn set_force(&mut self, force: f32, sampling_time: Duration) {
        let dt = sampling_time.as_secs_f32();
        let delta = force - self.force;
        let delta_rate = delta / dt;
        let delta = if delta_rate.abs() <= self.max_force_rate {
            delta
        } else {
            self.max_force_rate * delta.signum() * dt
        };
        self.force += delta;
        self.force = self.force.clamp(-self.max_force, self.max_force);
    }
}

impl Default for Pid {
    fn default() -> Self {
        Self {
            p: 0.0,
            i: 0.0,
            d: 0.0,
            kp: KP,
            ki: KI,
            kd: KD,
            prev_pos: None,
            target: TARGET,
        }
    }
}

pub struct Pid {
    p: f32,
    i: f32,
    d: f32,

    pub kp: f32,
    pub ki: f32,
    pub kd: f32,

    prev_pos: Option<f32>,
    pub target: f32,
}

impl Pid {
    pub fn reset(&mut self) {
        self.p = 0.0;
        self.i = 0.0;
        self.d = 0.0;
        self.prev_pos = None;
    }

    fn update(&mut self, pos: f32, sample_time: Duration) {
        let dt = sample_time.as_secs_f32();
        let error = self.target - pos;
        self.p = self.kp * error;
        self.i += self.ki * error;
        if let Some(prev_pos) = self.prev_pos {
            self.d = self.kd * (prev_pos - pos) / dt;
        }
        self.prev_pos = Some(pos);
    }

    fn total(&self) -> f32 {
        self.p + self.i + self.d
    }
}

pub struct Sensor {
    /// Random number generator for the noise
    rng: ThreadRng,
    /// Normal distribution of the noise
    normal: Normal<f32>,
}

impl Default for Sensor {
    fn default() -> Self {
        Self {
            rng: rand::thread_rng(),
            normal: Normal::new(0.0, NOISE).unwrap(),
        }
    }
}

impl Sensor {
    pub fn pos(&mut self, ball: &Ball) -> f32 {
        let noise = self.normal.sample(&mut self.rng);
        ball.pos + noise
    }

    pub fn set_sigma(&mut self, sigma: f32) {
        self.normal = Normal::new(0.0, sigma).unwrap();
    }

    pub fn reset(&mut self) {}
}
