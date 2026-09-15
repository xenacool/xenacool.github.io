use pystral_core::log::AvailableMove;

pub(super) fn select_nearest_axial_destination(
    reference: &AvailableMove,
    reachable: &[AvailableMove],
    direction: &str,
) -> Option<AvailableMove> {
    let candidate_key = |candidate: &AvailableMove| match direction {
        "left" if candidate.layer == reference.layer && candidate.hex.x < reference.hex.x => {
            Some((
                (candidate.hex.y - reference.hex.y).abs(),
                reference.hex.x - candidate.hex.x,
                candidate.hex.x,
                candidate.hex.y,
            ))
        }
        "right" if candidate.layer == reference.layer && candidate.hex.x > reference.hex.x => {
            Some((
                (candidate.hex.y - reference.hex.y).abs(),
                candidate.hex.x - reference.hex.x,
                candidate.hex.x,
                candidate.hex.y,
            ))
        }
        "up" if candidate.layer == reference.layer && candidate.hex.y < reference.hex.y => Some((
            (candidate.hex.x - reference.hex.x).abs(),
            reference.hex.y - candidate.hex.y,
            candidate.hex.x,
            candidate.hex.y,
        )),
        "down" if candidate.layer == reference.layer && candidate.hex.y > reference.hex.y => {
            Some((
                (candidate.hex.x - reference.hex.x).abs(),
                candidate.hex.y - reference.hex.y,
                candidate.hex.x,
                candidate.hex.y,
            ))
        }
        _ => None,
    };
    reachable
        .iter()
        .filter_map(|candidate| candidate_key(candidate).map(|key| (key, candidate)))
        .min_by_key(|(key, _)| *key)
        .map(|(_, candidate)| candidate.clone())
}

pub(super) fn select_nearest_layer_destination(
    reference: &AvailableMove,
    reachable: &[AvailableMove],
    direction: &str,
) -> Option<AvailableMove> {
    reachable
        .iter()
        .filter_map(|candidate| match direction {
            "layer-up" if candidate.layer > reference.layer => Some((
                candidate.layer - reference.layer,
                reference.hex.distance_to(candidate.hex),
                candidate.hex.x,
                candidate.hex.y,
                candidate,
            )),
            "layer-down" if candidate.layer < reference.layer => Some((
                reference.layer - candidate.layer,
                reference.hex.distance_to(candidate.hex),
                candidate.hex.x,
                candidate.hex.y,
                candidate,
            )),
            _ => None,
        })
        .min_by_key(|(layer_distance, hex_distance, q, r, _)| {
            (*layer_distance, *hex_distance, *q, *r)
        })
        .map(|(_, _, _, _, candidate)| candidate.clone())
}

pub(super) fn preview_path(
    source: &AvailableMove,
    destination: &AvailableMove,
) -> Vec<AvailableMove> {
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
    fn q_navigation_prefers_the_nearest_aligned_reachable_cell() {
        let reference = cell(0, 0);
        let aligned = cell(2, 0);
        let off_axis = cell(1, 1);
        assert_eq!(
            select_nearest_axial_destination(&reference, &[off_axis, aligned.clone()], "right"),
            Some(aligned)
        );
    }

    #[test]
    fn axial_navigation_uses_off_axis_then_stable_nearest_ties() {
        let reference = cell(0, 0);
        let first = cell(1, 1);
        let second = cell(2, -1);
        assert_eq!(
            select_nearest_axial_destination(&reference, &[second, first.clone()], "right"),
            Some(first)
        );
        assert_eq!(
            select_nearest_axial_destination(&reference, &[cell(-1, 0)], "right"),
            None
        );
    }

    #[test]
    fn r_navigation_and_layer_navigation_preserve_their_other_coordinates() {
        let reference = cell(0, 0);
        assert_eq!(
            select_nearest_axial_destination(&reference, &[cell(1, -2), cell(0, -1)], "up")
                .map(|candidate| candidate.hex),
            Some(hexx::Hex::new(0, -1))
        );
        let mut upper = cell(1, 0);
        upper.layer = 1;
        assert_eq!(
            select_nearest_layer_destination(&reference, &[upper.clone()], "layer-up"),
            Some(upper)
        );
    }

    #[test]
    fn opposite_directions_stop_at_bounds_and_ignore_other_layers() {
        let reference = cell(0, 0);
        let mut other_layer = cell(-1, 0);
        other_layer.layer = 1;
        assert_eq!(
            select_nearest_axial_destination(&reference, &[cell(1, 0), other_layer], "left"),
            None
        );
        assert_eq!(
            select_nearest_axial_destination(&reference, &[cell(0, 1), cell(1, -1)], "down")
                .map(|candidate| candidate.hex),
            Some(hexx::Hex::new(0, 1))
        );
    }
}
