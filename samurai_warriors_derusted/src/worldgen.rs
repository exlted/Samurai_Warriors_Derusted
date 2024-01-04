use bevy::prelude::ResMut;
use bevy_ecs_tilemap::prelude::{TilePos, TileTextureIndex};
use bevy_prng::ChaCha8Rng;
use bevy_rand::prelude::GlobalEntropy;
use crate::lifeform::Lifeform;
use crate::tile_data::TileTextureData;
use crate::utils::rand_range;

#[derive(Default)]
#[derive(PartialEq)]
pub enum IntersectState {
    #[default]
    None,
    Full,
    Partial
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Order {
    FirstGreater,
    SecondGreater,
    Same
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Copy)]
pub struct Rect {
    x: u8,
    y: u8,
    width: u8,
    height: u8
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Room {
    Basic {
        rect: Rect
    },
    Complex {
        rooms: Vec<Rect>,
        aabb: Rect
    }
}

impl Room {

    pub fn get_random_point_in(&self, rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>) -> (usize, usize) {
        match self {
            Room::Basic { rect } => {
                (rand_range((rect.x + 1) as u32, (rect.width - 2) as u32, rng) as usize,
                 rand_range((rect.y + 1) as u32, (rect.height - 2) as u32, rng) as usize)
            }
            Room::Complex { rooms, .. } => {
                let room_num = rand_range(0, rooms.len() as u32, rng) as usize;
                (rand_range((rooms[room_num].x + 1) as u32, (rooms[room_num].width - 2) as u32, rng) as usize,
                 rand_range((rooms[room_num].y + 1) as u32, (rooms[room_num].height - 2) as u32, rng) as usize)
            }
        }
    }

    fn blit(output: &mut Vec<Vec<TileTextureData>>, input: Vec<Vec<TileTextureData>>, x: usize, y: usize) {
        for input_x in 0..input.len() {
            for input_y in 0..input[input_x].len() {
                if input[input_x][input_y].can_replace(output[input_x + x][input_y + y]) {
                    output[input_x + x][input_y + y] = input[input_x][input_y];
                }
            }
        }
    }

    fn render_rect(rect: Rect) -> Vec<Vec<TileTextureData>> {
        let mut output = vec![];
        for x in 0..rect.width as usize {
            if x == 0 || x == (rect.width - 1) as usize {
                output.push(vec![TileTextureData::Wall{
                    connects_north: false,
                    connects_south: false,
                    connects_east: false,
                    connects_west: false
                }; rect.height as usize]);
            } else {
                output.push(vec![TileTextureData::Floor; rect.height as usize]);
                output[x][0] = TileTextureData::Wall {
                    connects_north: false,
                    connects_south: false,
                    connects_east: false,
                    connects_west: false
                };
                output[x][(rect.height - 1) as usize] = TileTextureData::Wall {
                    connects_north: false,
                    connects_south: false,
                    connects_east: false,
                    connects_west: false
                };
            }
        }
        output
    }

    pub fn render(self) -> Vec<Vec<TileTextureData>> {
        // For Complex Rooms:
        // Merge all the rooms into a single Vec of TileData (Only making Floors)
        // Then find the edges of the Floors and make Walls
        // Then the return value will be BLIT onto the map afterwards
        match self {
            Room::Basic { rect } => {
                Room::render_rect(rect)
            }
            Room::Complex { rooms, aabb } => {
                let mut output = vec![];
                for _ in 0..aabb.width as usize {
                    output.push(vec![TileTextureData::None; aabb.height as usize]);
                }

                for room in rooms {
                    let rendered_room = Room::render_rect(room);
                    let x_offset = (room.x - aabb.x) as usize;
                    let y_offset = (room.y - aabb.y) as usize;

                    Room::blit(&mut output, rendered_room, x_offset, y_offset);
                }
                output
            }
        }
    }

    fn calculate_aabb(rect1: Rect, rect2: Rect) -> Rect {
        let far_x = std::cmp::max(rect1.x + rect1.width, rect2.x + rect2.width);
        let far_y = std::cmp::max(rect1.y + rect1.height, rect2.y + rect2.height);
        let close_x = std::cmp::min(rect1.x, rect2.x);
        let close_y = std::cmp::min(rect1.y, rect2.y);

        let output = Rect {
            x: close_x,
            y: close_y,
            width: far_x - close_x,
            height: far_y - close_y
        };
        output
    }

    pub fn combine_rooms(this: Room, that: Room) -> Room {
        match (this.clone(), that.clone()) {
            (Room::Basic {rect: this_rect}, Room::Basic{rect: that_rect}) => {
                Room::Complex {
                    rooms: vec![this_rect.clone(), that_rect.clone()],
                    aabb: Room::calculate_aabb(this_rect, that_rect)
                }
            }
            (Room::Basic {rect}, Room::Complex {mut rooms, aabb}) => {
                rooms.push(rect.clone());
                let aabb = Room::calculate_aabb(rect, aabb);
                Room::Complex {
                    rooms,
                    aabb
                }
            }
            (Room::Complex {mut rooms, aabb}, Room::Basic {rect}) => {
                rooms.push(rect.clone());
                let aabb = Room::calculate_aabb(rect, aabb);
                Room::Complex {
                    rooms,
                    aabb
                }
            }
            (Room::Complex{rooms: mut this_rooms, aabb: this_aabb}, Room::Complex{rooms: mut that_rooms, aabb: that_aabb}) => {
                this_rooms.append(&mut that_rooms);
                let aabb = Room::calculate_aabb(this_aabb, that_aabb);
                Room::Complex {
                    rooms: this_rooms,
                    aabb
                }
            }
        }
    }

    fn edge_intersects(edge1_start: u8, edge1_end: u8, edge2_start: u8, edge2_end: u8) -> (IntersectState, Order) {
        // intersects if start/end of either edge is within the range of the other edge
        // fully intersects if start & end of either edge is fully within the range of the other edge
        let mut intersect_cnt_1 = 0;
        let mut intersect_cnt_2 = 0;
        let mut order = Order::FirstGreater;

        if edge1_start >= edge2_start && edge1_start <= edge2_end {
            intersect_cnt_1 += 1;
            order = Order::SecondGreater;
        }
        if edge1_end >= edge2_start && edge1_end <= edge2_end {
            intersect_cnt_1 += 1;
            order = Order::SecondGreater;
        }

        if edge2_start > edge1_start && edge2_start <= edge1_end {
            intersect_cnt_2 += 1;
            order = Order::SecondGreater;
        }
        if edge2_end >= edge1_start && edge2_end <= edge1_end {
            intersect_cnt_2 += 1;
            order = Order::SecondGreater;
        }

        let mut state = IntersectState::None;
        if intersect_cnt_1 == 1 || intersect_cnt_2 == 1 {
            state = IntersectState::Partial;
        } else if intersect_cnt_1 == 2 || intersect_cnt_2 == 2 {
            if intersect_cnt_1 == 2 && intersect_cnt_2 == 2 {
                order = Order::Same;
            } else if intersect_cnt_1 > intersect_cnt_2 {
                order = Order::FirstGreater;
            } else {
                order = Order::SecondGreater;
            }
            state = IntersectState::Full;
        }
        return (state, order);
    }

    fn basic_basic_intersection(this: Room, that: Room) -> (IntersectState, Order) {
        match (this, that) {
            (Room::Basic{ rect: this_rect }, Room::Basic{rect: that_rect}) => {
                let mut total_intersection = IntersectState::None;
                let mut total_order = Order::Same;

                let (x_intersection, x_order) = Room::edge_intersects(this_rect.x + 1, this_rect.x + that_rect.width - 1, that_rect.x + 1, that_rect.x + that_rect.width - 1);
                let (y_intersection, y_order) = Room::edge_intersects(this_rect.y + 1, this_rect.y + that_rect.height - 1, that_rect.y + 1, that_rect.y + that_rect.height - 1);

                match (x_intersection, y_intersection) {
                    (IntersectState::Partial, IntersectState::Partial) => {
                        total_intersection = IntersectState::Partial;
                        total_order = Order::Same;
                    }
                    (IntersectState::Partial, IntersectState::Full) => {
                        total_intersection = IntersectState::Partial;
                        total_order = Order::Same;
                    }
                    (IntersectState::Full, IntersectState::Partial) => {
                        total_intersection = IntersectState::Partial;
                        total_order = Order::Same;
                    }
                    (IntersectState::Full, IntersectState::Full) => {
                        total_intersection = IntersectState::Full;
                        if x_order == y_order {
                            total_order = x_order;
                        } else {
                            match (x_order, y_order) {
                                (Order::Same, total) => {
                                    total_order = total;
                                }
                                (total, Order::Same) => {
                                    total_order = total;
                                }
                                (_, _) => {
                                    panic!();
                                }
                            }
                        }
                    }
                    // Do nothing on purpose
                    _ => {}
                }

                (total_intersection, total_order)
            }
            _ => panic!()
        }
    }

    fn basic_complex_intersection(this: Room, that: Room) -> (IntersectState, Order) {
        match (this, that) {
            (Room::Basic{rect: this_rect}, Room::Complex{rooms: that_rooms, aabb: that_aabb}) => {
                let (mut intersection, mut order) = Room::basic_basic_intersection(Room::Basic{rect: this_rect.clone()}, Room::Basic{rect: that_aabb});

                // If the AABB doesn't intersect, no need to do more work
                if intersection != IntersectState::None {
                    // Reset states so they're meaningful again
                    intersection = IntersectState::None;
                    order = Order::Same;

                    // Do the more complex work
                    // Find all rooms that intersect with the compared basic room
                    for idx in 0..that_rooms.len() {
                        let that_room = that_rooms[idx];

                        let (inner_intersection, inner_order) = Room::basic_basic_intersection(Room::Basic{rect: this_rect.clone()}, Room::Basic{rect: that_room});
                        if inner_intersection == IntersectState::Full {
                            intersection = IntersectState::Full;
                            order = inner_order;
                            if inner_order == Order::SecondGreater {
                                break
                            }
                        } else if inner_intersection == IntersectState::Partial {
                            intersection = inner_intersection;
                            order = inner_order;
                        }
                    }
                }

                return (intersection, order);
            }
            _ => panic!()
        }
    }

    fn complex_complex_intersection(this: Room, that: Room) -> (IntersectState, Order) {
        match (this, that.clone()) {
            (Room::Complex{rooms: this_rooms, aabb: this_aabb}, Room::Complex{aabb: that_aabb, ..}) => {
                let (mut intersection, order) = Room::basic_basic_intersection(Room::Basic{rect: this_aabb}, Room::Basic{rect: that_aabb});

                // If the AABB doesn't intersect, no need  to do more work
                if intersection != IntersectState::None {
                    // Do the more complex work
                    for room in this_rooms {
                        // If there's any intersection between any 2 rooms between the two complex rooms, then it's an intersection!
                        let (inner_intersection, ..) = Room::basic_complex_intersection(Room::Basic{rect: room}, that.clone());
                        match inner_intersection {
                            IntersectState::None => {continue}
                            IntersectState::Full => {intersection = IntersectState::Partial; break; }
                            IntersectState::Partial => { intersection = IntersectState::Partial; break;}
                        }
                    }
                }

                (intersection, order)
            }
            _ => panic!()
        }
    }

    pub fn intersects(&self, other: Room) -> (IntersectState, Order) {
        match (self, &other) {
            (Room::Basic{..}, Room::Basic{..}) => {Room::basic_basic_intersection(self.clone(), other)}
            (Room::Basic{..}, Room::Complex{..}) => {Room::basic_complex_intersection(self.clone(), other)}
            (Room::Complex{..}, Room::Basic{..}) => {Room::basic_complex_intersection(other, self.clone())}
            (Room::Complex{..}, Room::Complex{..}) => {Room::complex_complex_intersection(self.clone(), other)}
        }
    }
}

// When we generate rooms, we'll check if the new room intersects with any old rooms
// If it does, we'll combine it with the room it intersects with... If it combines with multiple Complex Rooms... uh... try again?
// Then render into textures, woo!
#[derive(Clone)]
pub struct TerrainData {
    pub texture: TileTextureIndex,
    pub tile_data: TileTextureData
}

struct Corridor {
    pub from_x: usize,
    pub to_x: usize,
    pub from_y: usize,
    pub to_y: usize
}

impl Corridor {

    fn set_at(x: usize, y: usize, new_data: TileTextureData, output: &mut Vec<Vec<TileTextureData>>) {
        if new_data.can_replace(output[x][y]) {
            output[x][y] = new_data;
        }
    }

    const EMPTY_WALL: TileTextureData = TileTextureData::Wall{
        connects_north: false,
        connects_south: false,
        connects_east: false,
        connects_west: false,
    };

    fn is_extents(val: usize, min: usize, max: usize) -> bool {
        val == min || val == max - 1
    }

    fn render_corridor_tile(x: usize, y: usize, horizontal: bool, extents: bool, _start: bool, output: &mut Vec<Vec<TileTextureData>>) {
        if extents {
            Self::set_at(x, y, Self::EMPTY_WALL, output);
        } else {
            Self::set_at(x, y, TileTextureData::Floor, output);
        }
        if horizontal {
            Self::set_at(x, y - 1, Self::EMPTY_WALL, output);
            Self::set_at(x, y + 1, Self::EMPTY_WALL, output);
        } else {
            Self::set_at(x - 1, y, Self::EMPTY_WALL, output);
            Self::set_at(x + 1, y, Self::EMPTY_WALL, output);
        }
    }

    fn x_corridor(from_x: usize, to_x: usize, at_y: usize, output: &mut Vec<Vec<TileTextureData>>) {
        let min = std::cmp::min(from_x, to_x) - 1;
        let max = std::cmp::max(from_x, to_x) + 2;

        for x in min..max {
            Self::render_corridor_tile(x, at_y, true, Self::is_extents(x, min, max), Self::is_extents(x, min + 1, max - 1), output);
        }
    }

    fn y_corridor(from_y: usize, to_y: usize, at_x: usize, output: &mut Vec<Vec<TileTextureData>>) {
        let min = std::cmp::min(from_y, to_y) - 1;
        let max = std::cmp::max(from_y, to_y) + 2;

        for y in min..max {
            Self::render_corridor_tile(at_x, y, false, Self::is_extents(y, min, max), Self::is_extents(y, min + 1, max - 1), output);
        }
    }

    pub fn render(&self) -> Vec<Vec<TileTextureData>> {
        let (offset_x, offset_y) = self.get_offsets();
        let (extents_x, extents_y) = self.get_extents();
        let mut output = vec![vec![TileTextureData::None; extents_y]; extents_x];

        if self.from_x == self.to_x {
            Self::y_corridor(self.from_y - offset_y, self.to_y - offset_y, self.to_x - offset_x, &mut output);
        } else if self.from_y == self.to_y {
            Self::x_corridor(self.from_x - offset_x, self.to_x - offset_x, self.from_y - offset_y, &mut output);
        } else {
            Self::y_corridor(self.from_y - offset_y, self.to_y - offset_y, self.to_x - offset_x, &mut output);
            Self::x_corridor(self.from_x - offset_x, self.to_x - offset_x, self.from_y - offset_y, &mut output);
        }

        output
    }

    pub fn get_offsets(&self) -> (usize, usize) {
        (std::cmp::min(self.from_x, self.to_x) - 1, std::cmp::min(self.from_y, self.to_y) - 1)
    }

    fn get_extents(&self) -> (usize, usize) {
        let (offset_x, offset_y) = self.get_offsets();
        (std::cmp::max(self.from_x, self.to_x) + 2 - offset_x, std::cmp::max(self.from_y, self.to_y) + 2 - offset_y)
    }
}

pub(crate) struct RoomGenerator {
    pub room_count: u8,
    pub map_width: usize,
    pub map_height: usize,
    pub rooms: Vec<Room>,
    pub mean_room_width: u32,
    pub mean_room_height: u32,
    pub width_variance: u32,
    pub height_variance: u32,
    pub max_enemies_per_room: u32
}

impl RoomGenerator {

    fn check_neighbor(data: &Vec<Vec<TerrainData>>, x: usize, y: usize) -> (bool, bool) {
        // Return Value (Connect, Create)
        (data[x][y].tile_data.connects_to_walls(), data[x][y].tile_data.makes_walls())
    }

    fn check_neighbors(data: &mut Vec<Vec<TerrainData>>, x: usize, y: usize) -> (bool, bool) {
        let mut connects_north = false;
        let mut connects_south = false;
        let mut connects_east = false;
        let mut connects_west = false;
        let mut make_wall = false;

        let minus_x = x > 0;
        let minus_y = y > 0;
        let plus_x = x < (data.len() - 2);
        let plus_y = y < (data[x].len() - 1);

        if minus_x && minus_y {
            let (_, temp_make_wall) = RoomGenerator::check_neighbor(&data,x - 1, y - 1);
            make_wall = temp_make_wall || make_wall;
        }
        if minus_x {
            let (inner_connects_west, temp_make_wall) = RoomGenerator::check_neighbor(&data, x - 1, y);
            make_wall = make_wall || temp_make_wall;
            connects_west = inner_connects_west;
        }
        if minus_x && plus_y {
            let (_, temp_make_wall) = RoomGenerator::check_neighbor(&data,x - 1, y + 1);
            make_wall = make_wall || temp_make_wall;
        }
        if minus_y {
            let (inner_connects_south, temp_make_wall) = RoomGenerator::check_neighbor(&data, x, y - 1);
            make_wall = make_wall || temp_make_wall;
            connects_south = inner_connects_south;
        }
        if plus_y {
            let (inner_connects_north, temp_make_wall) = RoomGenerator::check_neighbor(&data, x, y + 1);
            make_wall = make_wall || temp_make_wall;
            connects_north = inner_connects_north;
        }
        if plus_x && minus_y{
            let (_, temp_make_wall) = RoomGenerator::check_neighbor(&data,x + 1, y - 1);
            make_wall = make_wall || temp_make_wall;
        }
        if plus_x {
            let (inner_connects_east, temp_make_wall) = RoomGenerator::check_neighbor(&data, x + 1, y);
            make_wall = make_wall || temp_make_wall;
            connects_east = inner_connects_east;
        }
        if plus_x && plus_y{
            let (_, temp_make_wall) = RoomGenerator::check_neighbor(&data,x + 1, y + 1);
            make_wall = make_wall || temp_make_wall;
        }
        if make_wall {
            data[x][y].tile_data = TileTextureData::Wall {
                connects_north,
                connects_east,
                connects_west,
                connects_south
            };
            return (connects_west, connects_south);
        } else {
            match data[x][y].tile_data {
                TileTextureData::Wall { .. } => {
                    data[x][y].tile_data = TileTextureData::None;
                }
                _=> {}
            }
        }
        (false, false)
    }

    fn blit(output: &mut Vec<Vec<TerrainData>>, input: Vec<Vec<TileTextureData>>, x: usize, y: usize) {
        for input_x in 0..input.len() {
            for input_y in 0..input[input_x].len() {
                if input[input_x][input_y] != TileTextureData::None {
                    if input[input_x][input_y].can_replace(output[input_x + x][input_y + y].tile_data) {
                        output[input_x + x][input_y + y].tile_data = input[input_x][input_y];
                    }
                }
            }
        }
    }

    fn generate_corridor(rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>, from_room: Room, to_room: Room, map: &mut Vec<Vec<TerrainData>>) {
        let (from_x, from_y) = from_room.get_random_point_in(rng);
        let (to_x, to_y) = to_room.get_random_point_in(rng);

        let corridor = Corridor {
            from_x,
            from_y,
            to_x,
            to_y
        };

        let (x_offset, y_offset) = corridor.get_offsets();
        RoomGenerator::blit(map, corridor.render(), x_offset, y_offset);
    }

    fn generate_random_entity(rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>, room: Room, entity_type: TileTextureData, output: &mut Vec<Lifeform>) {
        let (x, y) = room.get_random_point_in(rng);
        Self::generate_entity_at_point(rng, x, y, entity_type, output);
    }

    fn generate_entity_at_point (rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>, x: usize, y: usize, entity_type: TileTextureData, output: &mut Vec<Lifeform>) {
        output.push(Lifeform{
            texture: TerrainData{
                tile_data: entity_type,
                texture: TileTextureIndex(0)
            },
            position: TilePos {x: x as u32, y: y as u32},
            health: rand_range(48, 13, rng),
            strength: rand_range(5, 11, rng),
            defense: rand_range(0, 6, rng),
            level: 0,
            experience: 1
        });
    }

    pub fn generate_rooms(&mut self, rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>) -> (Vec<Vec<TerrainData>>, Vec<Lifeform>) {
        for _ in 0..self.room_count {
            let calc_width = rand_range(self.mean_room_width, self.width_variance, rng);
            let calc_height = rand_range(self.mean_room_height, self.height_variance, rng);
            let calc_x = rand_range(0, (self.map_width as u32) - calc_width, rng);
            let calc_y = rand_range(0, (self.map_height as u32) - calc_height, rng);

            let mut new_room = Room::Basic{
                rect: Rect {
                    x: calc_x as u8,
                    y: calc_y as u8,
                    width: calc_width as u8,
                    height: calc_height as u8
                }
            };

            let mut should_add = true;
            let mut remove_idx: Vec<usize> = vec![];
            for idx in 0..self.rooms.len() {
                let room = &self.rooms[idx];
                let (intersection_state, order) = room.intersects(new_room.clone());
                match intersection_state {
                    IntersectState::None => {
                        continue
                    }
                    IntersectState::Full => {
                        if order == Order::FirstGreater {
                            should_add = true;
                            remove_idx.push(idx);
                        } else {
                            should_add = false;
                            break;
                        }
                    }
                    IntersectState::Partial => {
                        should_add = true;
                        remove_idx.push(idx);
                        new_room = Room::combine_rooms(room.clone(), new_room.clone());
                    }
                }
            }
            let mut idx_removed = 0;
            for idx_to_remove in remove_idx {
                self.rooms.remove(idx_to_remove - idx_removed);
                idx_removed += 1;
            }
            if should_add {
                self.rooms.push(new_room);
            }
        }

        let mut output: Vec<Vec<TerrainData>> = vec![vec![TerrainData {
            texture: TileTextureIndex(0),
            tile_data: TileTextureData::None
            }; self.map_height]; self.map_width];
        let mut output_entities: Vec<Lifeform> = vec![];

        for idx in 0..self.rooms.len() {
            // Room Rendering
            let room = &self.rooms[idx];
            let rendered_room = room.clone().render();
            match room {
                Room::Basic { rect } => {
                    RoomGenerator::blit(&mut output, rendered_room, rect.x as usize, rect.y as usize);
                    let enemies_in_this_room = rand_range(0, self.max_enemies_per_room, rng);
                    for _ in 0..enemies_in_this_room {
                        Self::generate_random_entity(rng, Room::Basic{rect: *rect}, TileTextureData::Enemy, &mut output_entities);
                    }
                }
                Room::Complex { aabb, rooms } => {
                    RoomGenerator::blit(&mut output, rendered_room, aabb.x as usize, aabb.y as usize);
                    for idx_2 in 0..rooms.len() {
                        let room = rooms[idx_2];
                        let enemies_in_this_room = rand_range(0, self.max_enemies_per_room, rng);
                        for _ in 0..enemies_in_this_room {
                            Self::generate_random_entity(rng, Room::Basic{rect: room}, TileTextureData::Enemy, &mut output_entities);
                        }
                        if idx_2 != 0 {
                            Self::generate_corridor(rng, Room::Basic{rect: rooms[idx_2 - 1]}, Room::Basic{rect: rooms[idx_2]}, &mut output);
                        }
                    }
                }
            }

            // Entry/Exit generation
            if idx == 0 {
                let (entrance_x, entrance_y) = room.get_random_point_in(rng);
                RoomGenerator::blit(&mut output, vec![vec![TileTextureData::Entrance]], entrance_x as usize, entrance_y as usize);
                Self::generate_entity_at_point(rng, entrance_x, entrance_y, TileTextureData::Player, &mut output_entities);
            }
            if idx == self.rooms.len() - 1 {
                let (exit_x, exit_y) = room.get_random_point_in(rng);
                RoomGenerator::blit(&mut output, vec![vec![TileTextureData::Exit]], exit_x as usize, exit_y as usize);
            }

            // Corridor Generation
            if idx > 0 {
                // Generate a corridor from this room to the previous room
                Self::generate_corridor(rng, self.rooms[idx - 1].clone(), room.clone(), &mut output);
            }
        }

        for x in 0..output.len() {
            for y in 0..output[x].len() {
                match output[x][y].tile_data {
                    TileTextureData::Wall{..} => {
                        RoomGenerator::check_neighbors(&mut output, x, y);
                    }
                    _=> {}
                }

            }
        }

        (output, output_entities)
    }
}