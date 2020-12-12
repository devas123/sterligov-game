use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeMap;

const NEUTRAL: usize = 0;
const PURPLE: usize = 1;
const GREEN: usize = 2;
const ORANGE: usize = 3;
const YELLOW: usize = 4;
const RED: usize = 5;
const BLUE: usize = 6;
const POINT_COUNTS: [usize; 21] = [1, 2, 3, 4, 5, 16, 15, 14, 13, 12, 11, 12, 13, 14, 15, 16, 5, 4, 3, 2, 1];
const POINTS: &'static [&'static [usize]] = &[
    &[PURPLE],
    &[PURPLE, PURPLE],
    &[PURPLE, PURPLE, PURPLE],
    &[PURPLE, PURPLE, PURPLE, PURPLE],
    &[PURPLE, PURPLE, PURPLE, PURPLE, PURPLE],
    &[BLUE, BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN, GREEN],
    &[BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN],
    &[BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN],
    &[BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN],
    &[BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN],
    &[NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL],
    &[RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE],
    &[RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE],
    &[RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE],
    &[RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE],
    &[RED, RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE, ORANGE],
    &[YELLOW, YELLOW, YELLOW, YELLOW, YELLOW],
    &[YELLOW, YELLOW, YELLOW, YELLOW],
    &[YELLOW, YELLOW, YELLOW],
    &[YELLOW, YELLOW],
    &[YELLOW]
];

//180 places in total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    #[serde(serialize_with = "serialize_cones")]
    pub cones: HashMap<(usize, usize), usize>, //(row, position, user_id)
}

pub fn serialize_cones<S>(cones: &HashMap<(usize, usize), usize>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut map = serializer.serialize_map(Some(cones.len()))?;
    for ((row, col), v) in cones {
        map.serialize_entry(format!("{},{}", *row, *col).as_str(), v)?;
    }
    map.end()
}


impl GameState {
    pub fn add_cones_for_player(&self, player_number: usize, user_id: usize) -> std::result::Result<GameState, usize> {
        self.add_cones_for_color(player_number + 1, user_id)
    }

    fn add_cones_for_color(&self, color_number: usize, user_id: usize) -> std::result::Result<GameState, usize> {
        let mut gs = self.clone();
        for (row, cols) in POINTS.iter().enumerate() {
            for (col, color) in cols.iter().enumerate() {
                if *color == color_number {
                    gs.cones.insert((row, col), user_id);
                }
            }
        }
        Ok(gs)
    }

    pub fn get_cones(&self, user_id: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for (pair, uid) in &self.cones {
            if *uid == user_id {
                result.push(pair.clone())
            }
        }
        result
    }
    pub fn validate_dimensions(&self, row: i32, position: i32) -> std::result::Result<(usize, usize), usize> {
        if row < 0 || row > 20 {
            return Err(0);
        }
        if position < 0 || position >= POINT_COUNTS[row as usize] as i32 {
            return Err(0);
        }
        Ok((row as usize, position as usize))
    }

    pub fn validate_cone_position(&self, row: i32, position: i32, color: usize) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, position)?;
        if let Some(c) = self.cones.get(&(r, c)) {
            if *c != color {
                return Err(0);
            }
        } else {
            return Err(0);
        }
        Ok(true)
    }

    pub fn is_occupied(&self, row: i32, position: i32) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, position)?;
        return Ok(self.cones.get(&(r, c)).is_some());
    }

    pub fn add_cone(&mut self, row: i32, col: i32, color: usize) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, col)?;
        if self.cones.contains_key(&(r, c)) {
            return Err(0);
        }
        self.cones.insert((r, c).clone(), color.clone());
        Ok(true)
    }

    fn can_jump(&self, from: (i32, i32), to: (i32, i32)) -> std::result::Result<bool, usize> {
        let from_neighbors = self.get_neighbors(from.0, from.1)?;
        let to_valid = self.validate_dimensions(to.0, to.1)?;
       // println!("From {:?}, to {:?},  Neighbors: {:?}", from, to, from_neighbors);
        if from_neighbors.contains(&to_valid) || self.is_occupied(to.0, to.1)? {
         //   println!("Here1");
            return Err(1);
        }
        let mut common_neighbors = Vec::new();
        for x in from_neighbors {
            let nn = self.get_neighbors(x.0 as i32, x.1 as i32)?;
           // println!("x {:?},  Neighbors: {:?}", x, nn);
            if nn.contains(&(from.0 as usize, from.1 as usize)) && nn.contains(&(to.0 as usize, to.1 as usize)) {
                common_neighbors.push(x);
            }
        }
        // println!("common_neighbors: {:?}", common_neighbors);
        if common_neighbors.len() == 1 {
            let neighbor = common_neighbors.get(0).unwrap().clone();
            if self.is_occupied(neighbor.0 as i32, neighbor.1 as i32)? {
                return Ok(true);
            }
        }
        Err(1)
    }

    pub fn update_cones(&mut self, path: Vec<(i32, i32)>) -> std::result::Result<Vec<(usize, usize)>, usize> {
        self.validate_path(&path)?;
        let (s1, s2) = path[0].clone();
        let (e1, e2) = path[path.len() - 1].clone();
        let player = self.cones.remove(&(s1 as usize, s2 as usize));
        self.cones.insert((e1 as usize, e2 as usize), player.unwrap());
        Ok(path.clone().iter().map(|(x, y)| { (*x as usize, *y as usize) }).collect())
    }

    pub fn validate_path(&self, path: &Vec<(i32, i32)>) -> std::result::Result<bool, usize> {
        if path.len() < 2 {
            return Err(0);
        }
        if path.len() == 2 {
            let prev = *path.get(0).unwrap();
            let next = *path.get(1).unwrap();
            if self.is_occupied(next.0, next.1)? {
                return Err(0);
            }
            let neighbors = self.get_neighbors(prev.0, prev.1)?;
           // println!("Neighbors: {:?}", neighbors);
            return if neighbors.contains(&(next.0 as usize, next.1 as usize)) {
               // println!("shifting 1 position, ok");
                Ok(true)
            } else if self.can_jump(prev, next)? {
                Ok(true)
            } else {
                Err(0)
            };
        }
        for (ind, (row, col)) in path.iter().enumerate() {
            if ind == 0 {
                continue;
            }
            if self.is_occupied(*row, *col)? {
                return Err(1);
            }
            let prev = path[ind - 1];
            if !self.can_jump(prev, (*row, *col))? {
                return Err(1);
            }
        }
        Ok(true)
    }

    fn calculate_shift(up_shift: i32, incr: bool) -> (i32, i32) {
        let shift = if up_shift.abs() > 1 { up_shift.signum() * (up_shift.abs() - 1) / 2 } else { up_shift };
        if shift == -1 {
            (shift, shift + 1)
        } else if shift == 1 {
            (shift, shift - 1)
        } else if shift == -5 {
            if incr {
                (shift, shift + 1)
            } else {
                (shift, shift - 1)
            }
        } else if shift == 5 {
            if incr {
                (shift, shift + 1)
            } else {
                (shift, shift - 1)
            }
        } else {
            (-1, -1)
        }
    }

    pub fn get_neighbors(&self, row: i32, col: i32) -> std::result::Result<HashSet<(usize, usize)>, usize> {
        let (valid_row, _) = self.validate_dimensions(row, col)?;
        let last_row = (POINT_COUNTS.len() - 1) as i32;
        let upper_row_points_count = if row < last_row { POINT_COUNTS[valid_row + 1] as i32 } else { -1 };
        let lower_row_points_count = if row > 0 { POINT_COUNTS[valid_row - 1] as i32 } else { -1 };
        let current_row_points_count  = POINT_COUNTS[valid_row] as i32;
        let up_shift: i32 = if row < last_row { upper_row_points_count - current_row_points_count } else { 1 };
        let down_shift: i32 = if row > 0 { lower_row_points_count - current_row_points_count } else { 1 };
        let us = GameState::calculate_shift(up_shift, row <= 10);
        let ds = GameState::calculate_shift(down_shift, row >= 10);
        let all_pos: [(i32, i32); 6] = [(row, col - 1), (row, col + 1), (row - 1, col + ds.0), (row - 1, col + ds.1), (row + 1, col + us.0), (row + 1, col + us.1)];
        //println!("us: {:?}, ds: {:?}, all_pos: {:?}", us, ds, all_pos);
        let mut result = HashSet::new();
        for (r, c) in all_pos.iter() {
            if self.validate_dimensions(*r, *c).is_ok() {
                if *c != col || ((*r < valid_row as i32 && down_shift.abs() <= 1) || (*r > valid_row as i32 && up_shift.abs() <= 1)) {
                    result.insert((*r as usize, *c as usize));
                }
            }
        }
        Ok(result)
    }

    pub fn get_possible_steps(&self, pos: (i32, i32)) -> std::result::Result<Vec<(usize, usize)>, &'static str> {
        match self.validate_dimensions(pos.0, pos.1) {
            Ok(_) => {
                Ok(Vec::new())
            }
            Err(_) => {
                Err("wrong dimensions.")
            }
        }
    }

    pub fn new() -> GameState {
        GameState {
            cones: HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use super::*;

    #[test]
    fn test_validate_pos() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(5, 6, YELLOW).is_ok());
        assert!(game_state.add_cone(4, 6, YELLOW).is_err());
        assert!(game_state.validate_cone_position(5, 6, YELLOW).is_ok());
        assert_ne!(true, game_state.validate_cone_position(5, 8, YELLOW).is_ok());
        assert_ne!(true, game_state.validate_cone_position(4, 8, YELLOW).is_ok());
    }

    #[test]
    fn test_can_jump() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(4, 0, YELLOW).is_ok());
        assert!(game_state.can_jump((3, 0), (5, 5)).is_ok());
        assert!(game_state.can_jump((3, 0), (3, 1)).is_err());
        assert!(game_state.can_jump((3, 0), (3, 2)).is_err());

        assert!(game_state.add_cone(3, 1, YELLOW).is_ok());
        assert!(game_state.can_jump((3, 0), (3, 2)).is_ok());
        assert!(game_state.can_jump((3, 0), (5, 6)).is_err());
    }

    #[test]
    fn test_validate_path() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(4, 0, YELLOW).is_ok());
        assert!(game_state.validate_path(&vec![(3, 0), (5, 5)]).is_ok());
        assert!(game_state.validate_path(&vec![(3, 0), (5, 5), (5, 6)]).is_err());
        assert!(game_state.validate_path(&vec![(3, 0), (3, 1)]).is_ok());
        assert!(game_state.validate_path(&vec![(3, 1), (3, 0), (5, 5)]).is_err());
        assert!(game_state.validate_path(&vec![(3, 1)]).is_err());
        assert!(game_state.validate_path(&vec![(3, 1), (3, 1)]).is_err());
    }

    #[test]
    fn test_validate_path_regression() {
        let game_state = GameState::new().add_cones_for_color(PURPLE, 123).unwrap();
        assert!(game_state.validate_path(&vec![(3, 3), (5, 10)]).is_ok());
    }

    #[test]
    fn test_get_neighbors() {
        let game_state = GameState::new();
        assert_eq!(Ok(HashSet::from_iter(vec![(1, 0), (1, 1)].into_iter())), game_state.get_neighbors(0, 0));
        assert_eq!(Ok(HashSet::from_iter(vec![(12, 6), (12, 8), (11, 7), (11, 6), (13, 7), (13, 8)].into_iter())), game_state.get_neighbors(12, 7));
        assert_eq!(Ok(HashSet::from_iter(vec![(5, 1), (6, 0)].into_iter())), game_state.get_neighbors(5, 0));
        assert_eq!(Ok(HashSet::from_iter(vec![(5, 4), (5, 6), (4, 0), (6, 5), (6, 4)].into_iter())), game_state.get_neighbors(5, 5));
        assert_eq!(Ok(HashSet::from_iter(vec![(3, 3), (4, 3), (5, 9), (5, 10)].into_iter())), game_state.get_neighbors(4, 4));
        assert_eq!(Ok(HashSet::from_iter(vec![(3, 0), (3, 1), (4, 0), (4, 2), (5, 6), (5, 7)].into_iter())), game_state.get_neighbors(4, 1));
        assert_eq!(Ok(HashSet::from_iter(vec![(3, 2), (3, 3), (4, 2), (4, 4), (5, 9), (5, 8)].into_iter())), game_state.get_neighbors(4, 3));
        assert_eq!(Ok(HashSet::from_iter(vec![(14, 4), (14, 5), (15, 4), (15, 6), (16, 0)].into_iter())), game_state.get_neighbors(15, 5));
        assert_eq!(Ok(HashSet::from_iter(vec![(14, 9), (14, 10), (15, 9), (15, 11), (16, 4)].into_iter())), game_state.get_neighbors(15, 10));
        assert_eq!(Ok(HashSet::from_iter(vec![(9, 0), (9, 1), (11, 0), (11, 1), (10, 1)].into_iter())), game_state.get_neighbors(10, 0));
    }
}