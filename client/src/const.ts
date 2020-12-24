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
  export const generate_coords_up = (a, as, b, bs, c, cs, size) => {
    let res = [];
    for (let i = 0; i < size; i++) {
      res.push([[a, as + i], [b, bs + i], [c, cs + i]]);
    }
    return res;
  }
  export const small_triangles_coordinates = [
    [[1, 0], [1, 1], [2, 1]],
    [[2, 1], [2, 2], [3, 2]],
    [[2, 0], [2, 1], [3, 1]],
    
    [[3, 0], [3, 1], [4, 1]],
    [[3, 1], [3, 2], [4, 2]],
    [[3, 2], [3, 3], [4, 3]],
    
    [[4, 0], [4, 1], [5, 6]],
    [[4, 1], [4, 2], [5, 7]],
    [[4, 2], [4, 3], [5, 8]],
    [[4, 3], [4, 4], [5, 9]],

    ...generate_coords_up(5, 1, 6, 0, 6, 1, 14),
    ...generate_coords_up(6, 1, 7, 0, 7, 1, 13),
    ...generate_coords_up(7, 1, 8, 0, 8, 1, 12),

    ...generate_coords_up(8, 1, 9, 0, 9, 1, 4),
    ...generate_coords_up(8, 8, 9, 7, 9, 8, 4),

    ...generate_coords_up(9, 1, 10, 0, 10, 1, 3),
    ...generate_coords_up(9, 8, 10, 7, 10, 8, 3),

    ...generate_coords_up(10, 1, 11, 1, 11, 2, 3),
    ...generate_coords_up(10, 7, 11, 7, 11, 8, 3),

    ...generate_coords_up(11, 2, 12, 2, 12, 3, 3),
    ...generate_coords_up(11, 7, 12, 7, 12, 8, 3),


    ...generate_coords_up(12, 3, 13, 3, 13, 4, 7),
    ...generate_coords_up(13, 4, 14, 4, 14, 5, 6),
    ...generate_coords_up(14, 5, 15, 5, 15, 6, 5),

    ...generate_coords_up(15, 6, 16, 0, 16, 1, 4),
    ...generate_coords_up(16, 1, 17, 0, 17, 1, 3),
    ...generate_coords_up(17, 1, 18, 0, 18, 1, 2),
    ...generate_coords_up(18, 1, 19, 0, 19, 1, 1),

    ...generate_coords_up(11, 0, 11, 1, 12, 1, 1),
    ...generate_coords_up(12, 0, 12, 1, 13, 1, 2),
    ...generate_coords_up(13, 0, 13, 1, 14, 1, 3),
    ...generate_coords_up(14, 0, 14, 1, 15, 1, 4),


    ...generate_coords_up(11, 10, 11, 11, 12, 11, 1),
    ...generate_coords_up(12, 10, 12, 11, 13, 11, 2),
    ...generate_coords_up(13, 10, 13, 11, 14, 11, 3),
    ...generate_coords_up(14, 10, 14, 11, 15, 11, 4),

    [[9, 5], [9, 6], [10, 5]],
    ...generate_coords_up(10, 4, 10, 5, 11, 5, 2)
  ]

  export const small_triangles_center = [
    [[8,5],[9,4], [9,5]],
    [[8,7],[9,6], [9,7]],
    ...generate_coords_up(8, 5, 8, 6, 9, 5, 3),
    [[9,4], [10, 3], [10, 4]],
    ...generate_coords_up(9,4, 10, 3, 10, 4, 2),
    ...generate_coords_up(9,6, 10, 5, 10, 6, 2),
    ...generate_coords_up(10,4, 11, 4, 11, 5, 3),
    ...generate_coords_up(11,5, 12, 5, 12, 6, 2),
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
        case 2: return '#32BC23';
        case 3: return 'orange';
        case 4: return '#fbff2a';
        case 5: return 'red';
        case 6: return 'blue';
        default:
          return 'black';
      }
    }

    export function getContrast(color: number) {
      switch (color) {
        case 1: return 'white';
        case 2: return 'black';
        case 3: return 'black';
        case 4: return 'black';
        case 5: return 'white';
        case 6: return 'white';
        default:
          return 0;
      }
    }
    
    export function isInCorner(row: number, col: number) {
        return getColor(row, col) !== 'black';
    }

  export const X_USER_TOKEN = "X-User-Token";
  export const CONTENT_TYPE = "Content-Type";