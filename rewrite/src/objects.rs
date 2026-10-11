use bevy::prelude::Vec2;

/// Physical constants used by the source game's throwable/loose objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Properties {
    pub mass: f32,
    pub radius: f32,
    pub bounce: f32,
    pub friction: f32,
}

/// Result of resolving one overlapping pair of circular objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Collision {
    pub a_pos: Vec2,
    pub a_velocity: Vec2,
    pub b_pos: Vec2,
    pub b_velocity: Vec2,
}

/// Return the source item's mass, collision radius, bounce and friction.
///
/// Radii use the source display object's `(width + height) / 4`, except the
/// infant which explicitly sets its radius to 20. Unknown/non-physical names
/// receive a conservative baseball-like fallback; callers can still choose
/// to ignore collisions for those kinds.
pub fn properties(kind: &str) -> Properties {
    let k: String = kind
        .trim()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect();
    match k.as_str() {
        "baseball" | "baseballs" => p(0.3, 6.5, 0.2, 0.8),
        "bowlball" | "bowlingball" | "bowlingballs" => p(0.8, 11.0, 0.15, 0.9),
        "grenade" | "grenades" => p(0.35, 8.0, 0.2, 0.65),
        "mine" | "mines" => p(0.2, 7.0, 0.0, 0.0),
        "fireball" | "fireballs" => p(0.1, 7.5, 0.9, 0.9),
        "bouncyball" | "rubberball" | "rubberballs" => p(0.08, 4.0, 0.95, 0.95),
        "molotov" | "molotovcocktail" | "molotovcocktails" => p(0.45, 11.25, 0.5, 0.5),
        "baby" | "infant" | "infants" => p(0.5, 20.0, 0.1, 0.2),
        "radio" => p(0.375, 14.75, 0.2, 0.85),
        "orb" | "magicalorb" => p(0.05, 24.5, 0.85, 0.85),
        "vortex" | "gravityshifter" | "telekinesis" => p(0.5, 20.5, 0.9, 0.9),
        // Added after the source catalogue was matched: these reuse the proven
        // behaviour of an existing tool, so only the mass and size are new.
        "beachball" | "beachballs" => p(0.05, 11.0, 0.92, 0.98),
        "anvil" | "anvils" => p(2.0, 12.0, 0.03, 0.45),
        "duck" | "rubberduck" | "rubberducks" => p(0.06, 10.0, 0.65, 0.85),
        "marble" | "marbles" => p(0.08, 3.5, 0.85, 0.9),
        "snowball" | "snowballs" => p(0.12, 5.5, 0.35, 0.7),
        "basketball" | "basketballs" => p(0.32, 7.2, 0.72, 0.8),
        "waterballoon" | "waterballoons" => p(0.06, 6.0, 0.12, 0.9),
        "tomato" | "tomatoes" => p(0.18, 6.2, 0.3, 0.85),
        "firecracker" => p(0.1, 4.5, 0.3, 0.7),
        "dynamite" => p(0.4, 9.0, 0.15, 0.6),
        "cherrybombs" | "cherrybomb" => p(0.38, 8.5, 0.2, 0.6),
        "bomb" => p(0.8, 12.5, 0.1, 0.55),
        _ => p(0.3, 8.0, 0.2, 0.8),
    }
}

const fn p(mass: f32, radius: f32, bounce: f32, friction: f32) -> Properties {
    Properties {
        mass,
        radius,
        bounce,
        friction,
    }
}

/// Resolve two overlapping circles using the source `circleCollide` normal and
/// tangent decomposition. The source routine scales each outgoing normal
/// component by the object's own bounce value. This API has no bounce inputs,
/// so it uses a multiplier of 1; callers wanting source-perfect response must
/// apply each object's bounce value to its outgoing normal component.
///
/// Positions follow the source's midpoint separation (`r_sum * 0.5025`).
/// Coincident centers use the relative-velocity direction, or +X if both
/// velocities are identical, avoiding NaNs while remaining deterministic.
#[cfg(test)]
pub fn collide(
    a_pos: Vec2,
    a_velocity: Vec2,
    a_mass: f32,
    a_radius: f32,
    b_pos: Vec2,
    b_velocity: Vec2,
    b_mass: f32,
    b_radius: f32,
) -> Option<Collision> {
    collide_with_bounce(
        a_pos, a_velocity, a_mass, a_radius, b_pos, b_velocity, b_mass, b_radius, 1.0, 1.0,
    )
}

/// Source-compatible form of [`collide`] with each object's bounce factor.
/// The source `circleCollide` multiplies each resulting normal component by
/// `-object.bounce`; this preserves that exact sign convention.
pub fn collide_with_bounce(
    a_pos: Vec2,
    a_velocity: Vec2,
    a_mass: f32,
    a_radius: f32,
    b_pos: Vec2,
    b_velocity: Vec2,
    b_mass: f32,
    b_radius: f32,
    a_bounce: f32,
    b_bounce: f32,
) -> Option<Collision> {
    if !finite_vec(a_pos)
        || !finite_vec(a_velocity)
        || !finite_vec(b_pos)
        || !finite_vec(b_velocity)
        || !a_mass.is_finite()
        || !b_mass.is_finite()
        || !a_radius.is_finite()
        || !b_radius.is_finite()
        || !a_bounce.is_finite()
        || !b_bounce.is_finite()
        || a_mass <= 0.0
        || b_mass <= 0.0
        || a_radius <= 0.0
        || b_radius <= 0.0
    {
        return None;
    }
    let delta = a_pos - b_pos;
    let distance_sq = delta.length_squared();
    let sum_radius = a_radius + b_radius;
    if !distance_sq.is_finite() || distance_sq >= sum_radius * sum_radius {
        return None;
    }

    let normal = if distance_sq > 1.0e-12 {
        delta / distance_sq.sqrt()
    } else {
        let relative = a_velocity - b_velocity;
        let rel_sq = relative.length_squared();
        if rel_sq > 1.0e-12 && rel_sq.is_finite() {
            relative / rel_sq.sqrt()
        } else {
            Vec2::X
        }
    };
    // Source basis: normal n=(cos,sin), tangent=(-sin,cos).
    let tangent = Vec2::new(-normal.y, normal.x);
    let a_n = a_velocity.dot(normal);
    let b_n = b_velocity.dot(normal);
    let a_t = a_velocity.dot(tangent);
    let b_t = b_velocity.dot(tangent);
    // Weighted-mass normal response followed by the source's per-object
    // negative bounce multipliers.
    let total_mass = a_mass + b_mass;
    let a_out_n = ((a_mass - b_mass) * a_n + 2.0 * b_mass * b_n) / total_mass;
    let b_out_n = (2.0 * a_mass * a_n + (b_mass - a_mass) * b_n) / total_mass;
    let a_out = normal * (-a_out_n * a_bounce) + tangent * a_t;
    let b_out = normal * (-b_out_n * b_bounce) + tangent * b_t;

    let midpoint = (a_pos + b_pos) * 0.5;
    let separation = normal * (sum_radius * 0.5025);
    let result = Collision {
        a_pos: midpoint + separation,
        a_velocity: a_out,
        b_pos: midpoint - separation,
        b_velocity: b_out,
    };
    if finite_vec(result.a_pos)
        && finite_vec(result.a_velocity)
        && finite_vec(result.b_pos)
        && finite_vec(result.b_velocity)
    {
        Some(result)
    } else {
        None
    }
}

fn finite_vec(v: Vec2) -> bool {
    v.x.is_finite() && v.y.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1.0e-5;

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < EPS, "{a} != {b}");
    }

    #[test]
    fn equal_mass_head_on_matches_source_negative_bounce() {
        let c = collide_with_bounce(
            Vec2::new(-0.9, 0.0),
            Vec2::X,
            1.0,
            1.0,
            Vec2::new(0.9, 0.0),
            -Vec2::X,
            1.0,
            1.0,
            0.5,
            0.25,
        )
        .unwrap();
        // Source basis points from b to a, so this exact source sign damps
        // the approaching normal velocities rather than swapping them.
        close(c.a_velocity.x, 0.5);
        close(c.b_velocity.x, -0.25);
        close(c.a_pos.x, -1.005);
        close(c.b_pos.x, 1.005);
    }

    #[test]
    fn unequal_mass_response_matches_source_weighted_equations() {
        let c = collide_with_bounce(
            Vec2::new(-0.9, 0.0),
            Vec2::X,
            1.0,
            1.0,
            Vec2::new(0.9, 0.0),
            -Vec2::X,
            3.0,
            1.0,
            1.0,
            1.0,
        )
        .unwrap();
        // n points left; source pre-bounce outputs are +2 and 0 in that
        // basis, then each is negated as in `circleCollide`.
        close(c.a_velocity.x, 2.0);
        close(c.b_velocity.x, 0.0);
    }

    #[test]
    fn coincident_stationary_centers_stay_finite() {
        let c = collide(
            Vec2::ZERO,
            Vec2::ZERO,
            1.0,
            2.0,
            Vec2::ZERO,
            Vec2::ZERO,
            1.0,
            2.0,
        )
        .unwrap();
        assert!(finite_vec(c.a_pos) && finite_vec(c.b_pos));
        assert!(finite_vec(c.a_velocity) && finite_vec(c.b_velocity));
        close(c.a_pos.x, 2.01);
        close(c.b_pos.x, -2.01);
    }

    #[test]
    fn rubber_ducks_are_light_and_moderately_bouncy() {
        let duck = p(0.06, 10.0, 0.65, 0.85);
        for name in ["Rubber Ducks", "rubber-duck", "duck"] {
            assert_eq!(properties(name), duck);
        }
        assert_ne!(duck, properties("Beach Balls"));
        assert_ne!(duck, properties("Rubber Balls"));
    }

    #[test]
    fn source_item_properties_are_available() {
        assert_eq!(properties("Bowling Balls"), p(0.8, 11.0, 0.15, 0.9));
        assert_eq!(properties("infant"), p(0.5, 20.0, 0.1, 0.2));
        assert_eq!(properties("Rubber Balls"), p(0.08, 4.0, 0.95, 0.95));
    }
}
