export const NEUTRAL = 0;
	export const PURPLE = 1;
	export const GREEN = 2;
	export const ORANGE = 3;
	export const YELLOW = 4;
	export const RED = 5;
	export const BLUE = 6;
	export const colors = [
            [PURPLE],
            [PURPLE, PURPLE],
            [PURPLE, PURPLE, PURPLE],
            [PURPLE, PURPLE, PURPLE, PURPLE],
            [PURPLE, PURPLE, PURPLE, PURPLE, PURPLE],
            [BLUE, BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN, GREEN],
            [BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN],
            [BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN],
            [BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN],
            [BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN],
            [NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL],
            [RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE],
            [RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE],
            [RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE],
            [RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE],
            [RED, RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE, ORANGE],
            [YELLOW, YELLOW, YELLOW, YELLOW, YELLOW],
            [YELLOW, YELLOW, YELLOW, YELLOW],
            [YELLOW, YELLOW, YELLOW],
            [YELLOW, YELLOW],
            [YELLOW]
        ];
	export const pointCounts = [
	  1, 2, 3, 4, 5, 16,
	  15,14, 13, 12, 11,
	  12, 13, 14, 15,
	  16, 5,
	  4, 3, 2, 1
    ];
    
    export function getColor(row: number, col: number) {
		  let color = colors[row][col];
		  return getColorString(color);
    }

    export function getColorString(color: number) {
      switch (color) {
        case 1: return 'purple';
        case 2: return 'green';
        case 3: return 'orange';
        case 4: return 'yellow';
        case 5: return 'red';
        case 6: return 'blue';
        default:
          return 'black';
      }
    }

    export function getColorValue(color: number) {
      switch (color) {
        case 1: return 'purple';
        case 2: return 'green';
        case 3: return 'orange';
        case 4: return 'yellow';
        case 5: return 'red';
        case 6: return 'blue';
        default:
          return 'black';
      }
    }
    
    export function isInCorner(row: number, col: number) {
        return getColor(row, col) !== 'black';
    }

  export const X_USER_TOKEN = "X-User-Token";
  export const CONTENT_TYPE = "Content-Type";