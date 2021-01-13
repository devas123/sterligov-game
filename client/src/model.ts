export interface Player {
  user_id: number;
  color: number;
  name: string;
  ready: boolean;
}

export interface Move {
  by: Player,
  path: number[][],
  
}

export interface RoomDesc {
  id: string;
  name: string;
  winner: number;
  created_by: number;
  created_time: number;
  game_started: boolean;
  game_finished: boolean;
  active_player: number;
  number_of_player: number;
}
