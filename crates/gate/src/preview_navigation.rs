use pystral_core::log::AvailableMove;

const HEX_DIRECTIONS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

fn hex_direction_index(source: &AvailableMove, candidate: &AvailableMove) -> usize {
    let delta_q = candidate.hex.x - source.hex.x;
    let delta_r = candidate.hex.y - source.hex.y;
    HEX_DIRECTIONS
        .iter()
        .enumerate()
        .max_by_key(|(_, (q, r))| {
            // Dot product in the regular hex embedding. Plain axial dot
            // products make adjacent ring cells tie and do not give arrows
            // a stable clockwise/counter-clockwise ordering.
            (2 * delta_q + delta_r) * (2 * q + r) + 3 * delta_r * r
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub(super) fn select_rotated_destination(
    source: &AvailableMove,
    current: &AvailableMove,
    reachable: &[AvailableMove],
    clockwise: bool,
) -> Option<AvailableMove> {
    let current_direction = hex_direction_index(source, current);
    let current_distance = source.hex.distance_to(current.hex);
    for offset in 1..=6 {
        let direction = if clockwise {
            (current_direction + offset) % 6
        } else {
            (current_direction + 6 - offset) % 6
        };
        let mut candidates = reachable
            .iter()
            .filter(|candidate| {
                candidate.layer == current.layer
                    && hex_direction_index(source, candidate) == direction
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            (
                (source.hex.distance_to(candidate.hex) - current_distance).abs(),
                source.hex.distance_to(candidate.hex),
                candidate.hex.x,
                candidate.hex.y,
            )
        });
        if let Some(candidate) = candidates.into_iter().next() {
            return Some(candidate);
        }
    }
    None
}

pub(super) fn select_radial_destination(
    source: &AvailableMove,
    current: &AvailableMove,
    reachable: &[AvailableMove],
    away_from_source: bool,
) -> Option<AvailableMove> {
    let current_direction = hex_direction_index(source, current);
    let current_distance = source.hex.distance_to(current.hex);
    let mut candidates = reachable
        .iter()
        .filter(|candidate| {
            candidate.layer == current.layer
                && hex_direction_index(source, candidate) == current_direction
                && if away_from_source {
                    source.hex.distance_to(candidate.hex) > current_distance
                } else {
                    source.hex.distance_to(candidate.hex) < current_distance
                }
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        (
            source.hex.distance_to(candidate.hex),
            candidate.hex.x,
            candidate.hex.y,
        )
    });
    if away_from_source {
        candidates.into_iter().next()
    } else {
        candidates.pop()
    }
}

pub(super) fn preview_path(
    source: &AvailableMove,
    destination: Option<&AvailableMove>,
) -> Vec<AvailableMove> {
    let Some(destination) = destination else {
        return Vec::new();
    };
    let mut path = source
        .hex
        .line_to(destination.hex)
        .into_iter()
        .map(|hex| AvailableMove {
            hex,
            layer: source.layer,
            ap_cost: 0,
        })
        .collect::<Vec<_>>();
    if source.layer != destination.layer {
        path.push(AvailableMove {
            hex: destination.hex,
            layer: destination.layer,
            ap_cost: destination.ap_cost,
        });
    } else if let Some(last) = path.last_mut() {
        last.ap_cost = destination.ap_cost;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(q: i32, r: i32) -> AvailableMove {
        AvailableMove {
            hex: hexx::Hex::new(q, r),
            layer: 0,
            ap_cost: 1,
        }
    }

    #[test]
    fn horizontal_preview_navigation_rotates_around_source() {
        let source = cell(0, 0);
        let current = cell(1, 0);
        let reachable = vec![cell(1, 0), cell(1, -1), cell(0, -1), cell(-1, 0)];

        let next = select_rotated_destination(&source, &current, &reachable, true);

        assert_eq!(next.map(|move_| move_.hex), Some(hexx::Hex::new(1, -1)));
    }

    #[test]
    fn vertical_preview_navigation_changes_distance_on_current_ray() {
        let source = cell(0, 0);
        let current = cell(2, 0);
        let reachable = vec![cell(1, 0), cell(2, 0), cell(3, 0), cell(1, -1)];

        let closer = select_radial_destination(&source, &current, &reachable, false);
        let farther = select_radial_destination(&source, &current, &reachable, true);

        assert_eq!(closer.map(|move_| move_.hex), Some(hexx::Hex::new(1, 0)));
        assert_eq!(farther.map(|move_| move_.hex), Some(hexx::Hex::new(3, 0)));
    }
}
