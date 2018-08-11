/*
- Whiteout
- Strength is the number of max piles you can push
- Keys, multiple spots, opens door to house
- Snowmen, must defend with piles which they can break
- Power-ups and special items visible like keys
- Weather like blizzards and heat
- Accidental uncovering
- Day/night
*/
extern crate bear_lib_terminal;
extern crate rand;
extern crate froggy;
extern crate cgmath;

use std::fmt;
use std::mem;

use rand::{thread_rng, Rng};

use bear_lib_terminal::{
    terminal::{self, Event, KeyCode},
    Color
};

use froggy::{Storage, Pointer};

use cgmath::Vector2;

type Point = Vector2<i32>;

const WIDTH: i32 = 80;
const HEIGHT: i32 = 24;
const STATUS_HEIGHT: i32 = 1;

fn out_of_bounds(x: i32, y: i32) -> bool {
    x < 0 || x >= WIDTH || y < 0 || y >= HEIGHT
}

static WHITE: Color = Color { red: 255, green: 255, blue: 255, alpha: 255 };
static BLACK: Color = Color { red: 0, green: 0, blue: 0, alpha: 255 };
static RED: Color = Color { red: 255, green: 0, blue: 0, alpha: 255 };

const MAX_SNOW_PILE: i32 = 6;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Snow(i32);

impl Snow {
    fn is_clear(&self) -> bool {
        self.0 == 0
    }

    fn is_max_pile(&self) -> bool {
        self.0 == MAX_SNOW_PILE
    }

    fn pile_one(&mut self) {
        assert!(self.0 + 1 <= MAX_SNOW_PILE);
        self.0 += 1
    }

    fn take_all(&mut self) -> Snow {
        mem::replace(self, Snow(0))
    }

    fn take_needed(&mut self, snow: &mut Snow) -> i32 {
        let amm = MAX_SNOW_PILE - self.0;
        let amm = ::std::cmp::min(snow.0, amm);
        self.0 += amm;
        snow.0 -= amm;
        amm
    }
}

impl From<Snow> for char {
    fn from(snow: Snow) -> Self {
        match snow.0 {
            1 => '.',
            2 => '-',
            3 => ':',
            4 => '+',
            5 => '*',
            6 => '#',
            0 => ' ',
            _ => panic!("bug"),
        }
    }
}

impl From<Snow> for Color {
    fn from(snow: Snow) -> Self {
        match snow.0 {
            1 => WHITE,
            2 => WHITE,
            3 => WHITE,
            4 => WHITE,
            5 => WHITE,
            6 => WHITE,
            0 => BLACK,
            _ => panic!("bug"),
        }
    }
}

#[derive(Debug)]
enum ShovelState {
    Plowing,
    Shoveling,
}
use ShovelState::*;

impl fmt::Display for ShovelState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Plowing => write!(f, "\\_"),
            &Shoveling => write!(f, "->"),
        }
    }
}

struct Snowfield {
    piles: [Snow; WIDTH as usize * HEIGHT as usize]
}

impl Snowfield {
    fn new() -> Self {
        Self {
            piles: [Snow(0); WIDTH as usize * HEIGHT as usize],
        }
    }

    fn snow_at(&self, x: i32, y: i32) -> Snow {
        if out_of_bounds(x, y) {
            panic!("bug");
        }
        self.piles[y as usize * WIDTH as usize + x as usize]
    }

    fn snow_at_mut(&mut self, x: i32, y: i32) -> &mut Snow {
        if out_of_bounds(x, y) {
            panic!("bug");
        }
        &mut self.piles[y as usize * WIDTH as usize + x as usize]
    }
}

struct Whiteout {
    field: Snowfield ,
    player: Point,
    shovel: ShovelState,
    carrying: Snow,
}

impl Whiteout {
    fn new() -> Self {
        Self {
            field: Snowfield::new(),
            player: Point::new(WIDTH / 2, HEIGHT / 2),
            shovel: Plowing,
            carrying: Snow(0),
        }
    }

    fn randomized() -> Self {
        let mut wo = Self::new();
        for _ in 0..5000 {
            wo.flurry();
        }
        wo
    }

    fn flurry(&mut self) {
        const TRIES: i32 = 60;
        for _ in 0..TRIES {
            let x = thread_rng().gen_range(0, WIDTH);
            let y = thread_rng().gen_range(0, HEIGHT);

            let idx = y as usize * WIDTH as usize + x as usize;
            if !self.field.piles[idx].is_max_pile() {
                self.field.piles[idx].pile_one();
                return;
            }
        }
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let idx = y as usize * WIDTH as usize + x as usize;
                if !self.field.piles[idx].is_max_pile() {
                    self.field.piles[idx].pile_one();
                    return;
                }
            }
        }
    }

    fn print(&self) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let idx = y as usize * WIDTH as usize + x as usize;
                let snow = self.field.piles[idx];
                terminal::set_colors(snow.into(), BLACK);
                terminal::put_xy(x, y, snow.into());
            }
        }
        terminal::set_colors(if !self.carrying.is_clear() { WHITE } else { RED }, BLACK);
        terminal::print_xy(self.player.x, self.player.y, "@");
        terminal::set_colors(WHITE, BLACK);
        terminal::print_xy(
            0,
            HEIGHT,
            &format!("shovel: {}{}", self.shovel, if !self.carrying.is_clear() { '*' } else { ' ' }));
    }



    fn flip_shovel(&mut self) {
        if !self.carrying.is_clear() {
            let p = self.player;
            self.field.snow_at_mut(p.x, p.y).take_needed(&mut self.carrying);
            self.carrying = Snow(0);
        }
        self.shovel = match &self.shovel {
            &Shoveling => Plowing,
            &Plowing => Shoveling,
        };
    }

    fn can_push_snow(&self, origin: Point, delta: Point, strength: i32) -> bool {
        let source = origin + delta;
        if out_of_bounds(source.x, source.y) {
            return false;
        }
        let mut carry = self.field.snow_at(source.x, source.y);
        let mut pushing_max = carry.is_max_pile();
        let mut mult = 2;
        let mut target = origin + delta * mult;
        let mut strength_required = if pushing_max { 1 } else { 0 };

        loop {
            if out_of_bounds(target.x, target.y) {
                return false;
            }
            if carry.is_clear() {
                return true;
            }
            if strength_required > strength && pushing_max {
                return false;
            }

            let mut target_snow = self.field.snow_at(target.x, target.y);
            if target_snow.take_needed(&mut carry) == 0 {
                strength_required += 1;
            } else {
                pushing_max = false;
            }

            mult += 1;
            target = origin + delta * mult;
        }
    }

    fn push_snow(&mut self, origin: Point, delta: Point) {
        let source = origin + delta;
        let mut carry = self.field.snow_at_mut(source.x, source.y).take_all();
        let mut mult = 2;
        let mut target = origin + delta * mult;

        loop {
            self.field.snow_at_mut(target.x, target.y).take_needed(&mut carry);

            if carry.is_clear() {
                break;
            }

            mult += 1;
            target = origin + delta * mult;
        }
    }

    fn move_player(&mut self, delta: Point) {
        let target = self.player + delta;
        if out_of_bounds(target.x, target.y) {
            return;
        }

        self.player = match self.shovel {
            Plowing => {
                if self.can_push_snow(self.player, delta, 4) {
                    let p = self.player;
                    self.push_snow(p, delta);
                    target
                } else {
                    self.player
                }
            },
            Shoveling => {
                if out_of_bounds(target.x, target.y) {
                    self.player
                } else if self.field.snow_at(target.x, target.y).is_clear() {
                    target
                } else {
                    if self.carrying.is_clear() {
                        self.carrying = self.field.snow_at_mut(target.x, target.y).take_all();
                    } else if !self.field.snow_at(target.x, target.y).is_max_pile() {
                        self.field.snow_at_mut(target.x, target.y).take_needed(&mut self.carrying);
                    }
                    self.player
                }
            }
        }
    }

    fn update(&mut self) {
        for _ in 0..20 {
            self.flurry();
        }
    }
}

fn main() {
    terminal::open("Whiteout", WIDTH as u32, HEIGHT as u32 + STATUS_HEIGHT as u32);

    let mut wo = Whiteout::randomized();

    wo.print();
    terminal::refresh();
    while let Some(ev) = terminal::wait_event() {
        match ev {
            Event::Close => break,
            Event::KeyPressed { key: KeyCode::Space, ctrl: _, shift: _ } => wo.flip_shovel(),
            Event::KeyPressed { key: KeyCode::H, ctrl: _, shift: _ } => wo.move_player(Point::new(-1, 0)),
            Event::KeyPressed { key: KeyCode::J, ctrl: _, shift: _ } => wo.move_player(Point::new(0, 1)),
            Event::KeyPressed { key: KeyCode::K, ctrl: _, shift: _ } => wo.move_player(Point::new(0, -1)),
            Event::KeyPressed { key: KeyCode::L, ctrl: _, shift: _ } => wo.move_player(Point::new(1, 0)),
            _ => {}
        }
        wo.update();
        wo.print();
        terminal::refresh();
    }
    terminal::close();
}
