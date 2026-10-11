//! Explicit sprite identities; related physics do not imply identical artwork.
pub(crate) fn object_key(item: &str) -> &'static str {
    match item {
        "Grenades" => "grenade",
        "Molotov Cocktails" => "molotov",
        "Mines" => "mine",
        "Beach Balls" => "beachball",
        "Anvils" => "anvil",
        "Rubber Ducks" => "duck",
        "Baseballs" | "baseball" => "baseball",
        "Bowling Balls" | "bowlball" => "bowlball",
        "Rubber Balls" => "rubberball",
        "Infants" => "infant",
        "Radio" => "radio",
        "Magical Orb" | "Telekinesis" => "orb",
        "Dynamite" => "dynamite",
        "Bomb" => "bomb",
        "Cherry Bombs" => "cherrybomb",
        "Firecracker" => "firecracker",
        "Basketballs" => "basketball",
        "Tomatoes" => "tomato",
        "Snowballs" => "snowball",
        "Water Balloons" => "waterballoon",
        "Marbles" => "marble",
        "Gravity Shifter" => "vortex",
        "Missiles" => "missile",
        "Bazooka Rocket" => "bazooka_rocket",
        "Fireballs" | "fireball" => "fireball_custom",
        _ => "",
    }
}
pub(crate) fn gun_key(item: &str) -> &'static str {
    match item {
        "Machine Gun" => "machinegun_custom",
        "Shotgun" => "shotgun_custom",
        "Stun Gun" => "stunGun",
        "Bazooka" => "bazooka_launcher",
        _ => "pistol",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools_do_not_borrow_unrelated_sprites() {
        assert_ne!(object_key("Tomatoes"), object_key("Baseballs"));
        assert_ne!(object_key("Basketballs"), object_key("Baseballs"));
        assert_ne!(gun_key("Machine Gun"), gun_key("Pistol"));
        assert_ne!(gun_key("Shotgun"), gun_key("Pistol"));
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/tools");
        for item in [
            "Tomatoes",
            "Basketballs",
            "Snowballs",
            "Water Balloons",
            "Marbles",
            "Cherry Bombs",
            "Bomb",
            "Dynamite",
            "Firecracker",
            "Rubber Ducks",
        ] {
            assert!(
                assets.join(format!("{}.png", object_key(item))).is_file(),
                "missing art for {item}"
            );
        }
        for gun in ["Pistol", "Shotgun", "Machine Gun", "Stun Gun"] {
            assert!(
                assets.join(format!("{}.png", gun_key(gun))).is_file(),
                "{gun}"
            );
        }
    }
}
