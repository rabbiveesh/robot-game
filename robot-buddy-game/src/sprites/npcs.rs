use crate::prelude::*;

const TS: f32 = 48.0;

pub fn draw_mommy(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;

    // Shadow
    draw_ellipse(cx, y + TS - 4.0, 12.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Dress (trapezoid as rectangle — close enough)
    draw_rectangle(cx - 12.0, cy - 2.0, 24.0, 20.0, Color::from_rgba(224, 64, 251, 255));

    // Head
    draw_circle(cx, cy - 8.0, 10.0, Color::from_rgba(255, 204, 128, 255));

    // Hair
    let hair = Color::from_rgba(78, 52, 46, 255);
    draw_circle(cx, cy - 10.0, 11.0, hair);
    draw_rectangle(cx - 11.0, cy - 4.0, 22.0, 8.0, Color::from_rgba(255, 204, 128, 255)); // trim hair bottom
    draw_rectangle(cx - 11.0, cy - 8.0, 4.0, 16.0, hair); // left side hair
    draw_rectangle(cx + 7.0, cy - 8.0, 4.0, 16.0, hair);  // right side hair

    // Eyes
    let eye = Color::from_rgba(51, 51, 51, 255);
    draw_rectangle(cx - 5.0, cy - 10.0, 3.0, 3.0, eye);
    draw_rectangle(cx + 2.0, cy - 10.0, 3.0, 3.0, eye);

    // Smile
    draw_line(cx - 4.0, cy - 4.0, cx + 4.0, cy - 4.0, 1.5, Color::from_rgba(233, 30, 99, 255));

    // Heart
    let heart_bob = (time * 2.0).sin() * 2.0;
    draw_circle(cx - 2.0, cy - 24.0 + heart_bob, 3.0, Color::from_rgba(233, 30, 99, 255));
    draw_circle(cx + 2.0, cy - 24.0 + heart_bob, 3.0, Color::from_rgba(233, 30, 99, 255));
}

pub fn draw_sage(x: f32, y: f32, _time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;

    // Shadow
    draw_ellipse(cx, y + TS - 4.0, 12.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Robe
    draw_rectangle(cx - 11.0, cy - 4.0, 22.0, 22.0, Color::from_rgba(126, 87, 194, 255));

    // Head
    draw_circle(cx, cy - 8.0, 9.0, Color::from_rgba(255, 204, 128, 255));

    // Wizard hat
    let hat = Color::from_rgba(126, 87, 194, 255);
    // Hat triangle (approximated with rectangle + smaller rect)
    draw_rectangle(cx - 6.0, cy - 30.0, 12.0, 18.0, hat);
    draw_rectangle(cx - 3.0, cy - 34.0, 6.0, 6.0, hat);
    // Hat brim
    draw_rectangle(cx - 14.0, cy - 14.0, 28.0, 4.0, hat);
    // Star on hat
    draw_text("\u{2605}", cx - 5.0, cy - 18.0, 12.0, Color::from_rgba(255, 213, 79, 255));

    // Eyes
    let eye = Color::from_rgba(51, 51, 51, 255);
    draw_rectangle(cx - 5.0, cy - 10.0, 3.0, 3.0, eye);
    draw_rectangle(cx + 2.0, cy - 10.0, 3.0, 3.0, eye);

    // Beard
    draw_triangle(
        vec2(cx - 5.0, cy - 2.0),
        vec2(cx + 5.0, cy - 2.0),
        vec2(cx, cy + 10.0),
        Color::from_rgba(224, 224, 224, 255),
    );
}

pub fn draw_shopkeeper(x: f32, y: f32, _time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;

    // Shadow
    draw_ellipse(cx, y + TS - 4.0, 12.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Apron / robe (pink)
    draw_rectangle(cx - 11.0, cy - 4.0, 22.0, 22.0, Color::from_rgba(233, 30, 99, 255));

    // Head
    draw_circle(cx, cy - 8.0, 9.0, Color::from_rgba(255, 204, 128, 255));

    // Shopkeeper hat (pink beret-style, no wizard point)
    let hat = Color::from_rgba(233, 30, 99, 255);
    draw_rectangle(cx - 10.0, cy - 18.0, 20.0, 8.0, hat);
    draw_rectangle(cx - 12.0, cy - 12.0, 24.0, 4.0, hat);
    // Coin emblem on hat
    draw_circle(cx, cy - 15.0, 4.0, Color::from_rgba(255, 213, 79, 255));
    draw_text("$", cx - 3.0, cy - 12.0, 10.0, Color::from_rgba(233, 30, 99, 255));

    // Eyes
    let eye = Color::from_rgba(51, 51, 51, 255);
    draw_rectangle(cx - 5.0, cy - 10.0, 3.0, 3.0, eye);
    draw_rectangle(cx + 2.0, cy - 10.0, 3.0, 3.0, eye);

    // Smile (wider, friendlier)
    draw_line(cx - 4.0, cy - 3.0, cx + 4.0, cy - 3.0, 1.5, Color::from_rgba(51, 51, 51, 255));
    // Mustache
    draw_line(cx - 6.0, cy - 5.0, cx - 1.0, cy - 4.0, 1.5, Color::from_rgba(78, 52, 46, 255));
    draw_line(cx + 1.0, cy - 4.0, cx + 6.0, cy - 5.0, 1.5, Color::from_rgba(78, 52, 46, 255));
}

pub fn draw_kid(x: f32, y: f32, hair_color: Color, shirt_color: Color, pigtails: bool, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 6.0;
    let bob = (time * 3.0).sin() * 1.0;

    // Shadow
    draw_ellipse(cx, y + TS - 3.0, 9.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Body
    draw_rectangle(cx - 7.0, cy - 1.0 + bob, 14.0, 12.0, shirt_color);

    // Head (bigger relative to body)
    draw_circle(cx, cy - 7.0 + bob, 9.0, Color::from_rgba(255, 204, 128, 255));

    // Hair
    draw_circle(cx, cy - 9.0 + bob, 10.0, hair_color);
    draw_rectangle(cx - 10.0, cy - 3.0 + bob, 20.0, 8.0, Color::from_rgba(255, 204, 128, 255));

    if pigtails {
        draw_circle(cx - 11.0, cy - 5.0 + bob, 4.0, hair_color);
        draw_circle(cx + 11.0, cy - 5.0 + bob, 4.0, hair_color);
    }

    // Eyes
    let eye = Color::from_rgba(51, 51, 51, 255);
    draw_rectangle(cx - 3.0, cy - 8.0 + bob, 2.0, 2.0, eye);
    draw_rectangle(cx + 1.0, cy - 8.0 + bob, 2.0, 2.0, eye);

    // Smile
    draw_line(cx - 2.0, cy - 4.0 + bob, cx + 2.0, cy - 4.0 + bob, 1.0, eye);
}

pub fn draw_dog(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 6.0;
    let wag = (time * 8.0).sin() * 4.0;

    // Shadow
    draw_ellipse(cx, y + TS - 3.0, 10.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Body
    draw_rectangle(cx - 10.0, cy - 2.0, 20.0, 12.0, Color::from_rgba(141, 110, 99, 255));

    // Head
    draw_circle(cx, cy - 8.0, 8.0, Color::from_rgba(161, 136, 127, 255));

    // Ears
    draw_rectangle(cx - 9.0, cy - 14.0, 5.0, 8.0, Color::from_rgba(121, 85, 72, 255));
    draw_rectangle(cx + 4.0, cy - 14.0, 5.0, 8.0, Color::from_rgba(121, 85, 72, 255));

    // Eyes (glitchy for doghouse)
    let glitch = ((time * 10.0).sin() * 127.0 + 128.0) as u8;
    draw_rectangle(cx - 4.0, cy - 10.0, 3.0, 3.0, Color::from_rgba(glitch, 255 - glitch, 0, 255));
    draw_rectangle(cx + 1.0, cy - 10.0, 3.0, 3.0, Color::from_rgba(255 - glitch, glitch, 0, 255));

    // Nose
    draw_circle(cx, cy - 4.0, 2.0, Color::from_rgba(51, 51, 51, 255));

    // Tail (wagging)
    draw_line(cx + 10.0, cy, cx + 14.0 + wag, cy - 6.0, 2.0, Color::from_rgba(141, 110, 99, 255));

    // Legs
    let leg = Color::from_rgba(121, 85, 72, 255);
    draw_rectangle(cx - 8.0, cy + 10.0, 4.0, 6.0, leg);
    draw_rectangle(cx + 4.0, cy + 10.0, 4.0, 6.0, leg);
}

/// Friendly reef shark — the gate guardian. Big toothy grin, never menacing.
/// `asleep` droops the eyes into happy arcs (napping across the path).
pub fn draw_shark(x: f32, y: f32, time: f32, asleep: bool) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = (time * 1.5).sin() * 1.5;
    let body = Color::from_rgba(96, 125, 139, 255);
    let belly = Color::from_rgba(207, 216, 220, 255);

    // Water shadow
    draw_ellipse(cx, y + TS - 4.0, 14.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Tail (swishing)
    let swish = (time * 3.0).sin() * 4.0;
    draw_triangle(
        vec2(cx - 14.0, cy + bob),
        vec2(cx - 22.0, cy - 6.0 + bob + swish),
        vec2(cx - 22.0, cy + 8.0 + bob - swish),
        body,
    );

    // Body
    draw_ellipse(cx, cy + bob, 15.0, 9.0, 0.0, body);
    draw_ellipse(cx, cy + 3.0 + bob, 13.0, 5.0, 0.0, belly);

    // Dorsal fin
    draw_triangle(
        vec2(cx - 2.0, cy - 8.0 + bob),
        vec2(cx + 6.0, cy - 8.0 + bob),
        vec2(cx + 2.0, cy - 18.0 + bob),
        body,
    );

    // Eye (happy)
    let eye = Color::from_rgba(33, 33, 33, 255);
    if asleep {
        draw_line(cx + 5.0, cy - 3.0 + bob, cx + 11.0, cy - 3.0 + bob, 1.5, eye);
    } else {
        draw_circle(cx + 8.0, cy - 3.0 + bob, 2.2, eye);
        draw_circle(cx + 8.8, cy - 3.8 + bob, 0.8, WHITE);
    }

    // Big friendly grin with little teeth
    draw_line(cx + 4.0, cy + 4.0 + bob, cx + 15.0, cy + 2.0 + bob, 1.5, eye);
    for i in 0..3 {
        let tx = cx + 6.0 + i as f32 * 3.5;
        draw_triangle(
            vec2(tx, cy + 3.5 + bob),
            vec2(tx + 2.5, cy + 3.5 + bob),
            vec2(tx + 1.25, cy + 6.0 + bob),
            WHITE,
        );
    }

    if asleep {
        // floating "z"s
        let zf = (time * 1.5).sin() * 2.0;
        draw_text("z", cx + 12.0, cy - 12.0 + zf, 14.0, Color::from_rgba(255, 255, 255, 200));
        draw_text("Z", cx + 18.0, cy - 20.0 - zf, 18.0, Color::from_rgba(255, 255, 255, 160));
    }
}

/// Sea turtle — a calm wandering guide. Domed shell, paddling flippers.
pub fn draw_sea_turtle(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 6.0;
    let paddle = (time * 2.5).sin() * 3.0;

    draw_ellipse(cx, y + TS - 3.0, 13.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Flippers
    let flip = Color::from_rgba(56, 142, 100, 255);
    draw_ellipse(cx - 11.0, cy + 2.0 + paddle, 6.0, 3.0, 0.6, flip);
    draw_ellipse(cx + 11.0, cy + 2.0 - paddle, 6.0, 3.0, -0.6, flip);

    // Shell
    draw_ellipse(cx, cy, 14.0, 10.0, 0.0, Color::from_rgba(94, 122, 60, 255));
    // Shell plates
    let plate = Color::from_rgba(120, 150, 80, 255);
    draw_circle(cx, cy - 1.0, 4.0, plate);
    for i in 0..5 {
        let a = i as f32 * std::f32::consts::TAU / 5.0;
        draw_circle(cx + a.cos() * 8.0, cy - 1.0 + a.sin() * 5.0, 2.5, plate);
    }

    // Head
    draw_circle(cx, cy - 11.0, 5.0, Color::from_rgba(76, 175, 110, 255));
    let eye = Color::from_rgba(33, 33, 33, 255);
    draw_circle(cx - 2.0, cy - 12.0, 1.2, eye);
    draw_circle(cx + 2.0, cy - 12.0, 1.2, eye);
    draw_line(cx - 2.0, cy - 9.0, cx + 2.0, cy - 9.0, 1.0, eye);
}

/// Dolphin — a playful wanderer arcing through the water.
pub fn draw_dolphin(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let arc = (time * 2.0).sin() * 3.0;
    let body = Color::from_rgba(120, 170, 200, 255);
    let belly = Color::from_rgba(235, 245, 250, 255);

    draw_ellipse(cx, y + TS - 4.0, 13.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Body (curved)
    draw_ellipse(cx, cy + arc, 14.0, 7.0, 0.0, body);
    draw_ellipse(cx + 1.0, cy + 3.0 + arc, 11.0, 4.0, 0.0, belly);

    // Snout
    draw_triangle(
        vec2(cx + 11.0, cy - 2.0 + arc),
        vec2(cx + 20.0, cy + 1.0 + arc),
        vec2(cx + 11.0, cy + 3.0 + arc),
        body,
    );

    // Dorsal fin (curved back)
    draw_triangle(
        vec2(cx - 2.0, cy - 6.0 + arc),
        vec2(cx + 5.0, cy - 6.0 + arc),
        vec2(cx - 5.0, cy - 15.0 + arc),
        body,
    );

    // Tail fluke
    let swish = (time * 3.0).cos() * 3.0;
    draw_triangle(
        vec2(cx - 13.0, cy + arc),
        vec2(cx - 21.0, cy - 5.0 + arc + swish),
        vec2(cx - 21.0, cy + 5.0 + arc - swish),
        body,
    );

    // Eye + smile
    let eye = Color::from_rgba(33, 33, 33, 255);
    draw_circle(cx + 7.0, cy - 2.0 + arc, 1.6, eye);
    draw_line(cx + 8.0, cy + 2.0 + arc, cx + 15.0, cy + 1.0 + arc, 1.2, eye);
}

/// Little crab — ambient skittering wanderer.
pub fn draw_crab(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 8.0;
    let scuttle = (time * 6.0).sin() * 1.5;
    let shell = Color::from_rgba(229, 78, 52, 255);

    draw_ellipse(cx, y + TS - 3.0, 9.0, 3.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Legs
    let leg = Color::from_rgba(180, 50, 30, 255);
    for i in 0..3 {
        let ly = cy + i as f32 * 3.0;
        draw_line(cx - 8.0, ly, cx - 13.0, ly + 2.0 + scuttle, 1.5, leg);
        draw_line(cx + 8.0, ly, cx + 13.0, ly + 2.0 - scuttle, 1.5, leg);
    }

    // Body
    draw_ellipse(cx, cy, 9.0, 6.0, 0.0, shell);

    // Claws
    draw_circle(cx - 11.0, cy - 4.0 + scuttle, 3.0, shell);
    draw_circle(cx + 11.0, cy - 4.0 - scuttle, 3.0, shell);

    // Eyes on stalks
    let eye = Color::from_rgba(33, 33, 33, 255);
    draw_line(cx - 3.0, cy - 5.0, cx - 3.0, cy - 9.0, 1.0, shell);
    draw_line(cx + 3.0, cy - 5.0, cx + 3.0, cy - 9.0, 1.0, shell);
    draw_circle(cx - 3.0, cy - 9.0, 1.5, eye);
    draw_circle(cx + 3.0, cy - 9.0, 1.5, eye);
}

/// Jellyfish — drifting ambient critter with a pulsing bell and tendrils.
pub fn draw_jellyfish(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + (time * 1.2).sin() * 3.0;
    let pulse = (time * 3.0).sin() * 1.5;
    let bell = Color::from_rgba(206, 147, 216, 180);

    // Bell
    draw_circle(cx, cy, 9.0 + pulse, bell);
    draw_rectangle(cx - 9.0 - pulse, cy, 18.0 + pulse * 2.0, 5.0, bell);

    // Tendrils
    let tendril = Color::from_rgba(186, 104, 200, 160);
    for i in 0..4 {
        let tx = cx - 6.0 + i as f32 * 4.0;
        let wig = (time * 2.5 + i as f32).sin() * 3.0;
        draw_line(tx, cy + 4.0, tx + wig, cy + 16.0, 1.5, tendril);
    }

    // Eyes
    let eye = Color::from_rgba(74, 20, 90, 255);
    draw_circle(cx - 3.0, cy, 1.3, eye);
    draw_circle(cx + 3.0, cy, 1.3, eye);
}

/// A friendly little alien. `hue` lets reef-style rosters drop in green pals or
/// a rusty-red Mars guardian without a new sprite. Floaty bob + waving antennae.
pub fn draw_alien(x: f32, y: f32, time: f32, body: Color) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0 + (time * 1.6).sin() * 1.5;

    draw_ellipse(cx, y + TS - 3.0, 11.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Body / belly
    draw_ellipse(cx, cy + 2.0, 10.0, 11.0, 0.0, body);
    draw_ellipse(cx, cy + 5.0, 6.0, 6.0, 0.0, Color::new(1.0, 1.0, 1.0, 0.18));

    // Big head
    draw_circle(cx, cy - 8.0, 9.0, body);

    // Antennae with wobbling tips
    let wob = (time * 3.0).sin() * 2.0;
    let tip = Color::from_rgba(255, 235, 120, 255);
    draw_line(cx - 4.0, cy - 14.0, cx - 6.0 + wob, cy - 22.0, 1.5, body);
    draw_line(cx + 4.0, cy - 14.0, cx + 6.0 - wob, cy - 22.0, 1.5, body);
    draw_circle(cx - 6.0 + wob, cy - 22.0, 2.0, tip);
    draw_circle(cx + 6.0 - wob, cy - 22.0, 2.0, tip);

    // One big friendly eye + a smaller one
    draw_circle(cx - 2.0, cy - 8.0, 4.0, WHITE);
    draw_circle(cx + 5.0, cy - 9.0, 2.5, WHITE);
    draw_circle(cx - 2.0, cy - 8.0, 1.8, Color::from_rgba(33, 33, 33, 255));
    draw_circle(cx + 5.0, cy - 9.0, 1.2, Color::from_rgba(33, 33, 33, 255));

    // Smile
    draw_line(cx - 3.0, cy - 2.0, cx + 3.0, cy - 2.0, 1.2, Color::from_rgba(33, 33, 33, 255));

    // Little arms
    draw_line(cx - 9.0, cy + 1.0, cx - 13.0, cy - 2.0 + wob, 2.0, body);
    draw_line(cx + 9.0, cy + 1.0, cx + 13.0, cy - 2.0 - wob, 2.0, body);
}

/// Fuel depot — a chunky droid with a pulsing gauge. Refuels the rocket when
/// the kid solves its math.
pub fn draw_fuel_depot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 2.0;
    let metal = Color::from_rgba(120, 130, 150, 255);
    let dark = Color::from_rgba(70, 78, 96, 255);

    draw_ellipse(cx, y + TS - 3.0, 13.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Tank body
    draw_rectangle(cx - 11.0, cy - 8.0, 22.0, 24.0, metal);
    draw_rectangle_lines(cx - 11.0, cy - 8.0, 22.0, 24.0, 2.0, dark);
    // Cap
    draw_rectangle(cx - 7.0, cy - 13.0, 14.0, 6.0, dark);

    // Pulsing fuel gauge (green = full vibes)
    let pulse = (time * 3.0).sin() * 0.5 + 0.5;
    draw_rectangle(cx - 7.0, cy - 3.0, 14.0, 8.0, Color::from_rgba(20, 24, 34, 255));
    draw_rectangle(cx - 6.0, cy - 2.0, 12.0 * (0.4 + pulse * 0.6), 6.0,
        Color::from_rgba(102, 220, 120, 255));
    // Nozzle + hose
    draw_line(cx + 11.0, cy + 2.0, cx + 16.0, cy + 6.0, 2.5, dark);
    // "F" label
    draw_text("F", cx - 3.0, cy + 13.0, 14.0, Color::from_rgba(255, 235, 120, 255));
}

/// Star-chart terminal — a glowing console with a tiny constellation that the
/// asteroid-base keeper uses to run pattern puzzles.
pub fn draw_star_terminal(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 2.0;
    let frame = Color::from_rgba(90, 100, 124, 255);
    let screen = Color::from_rgba(18, 24, 48, 255);

    draw_ellipse(cx, y + TS - 3.0, 12.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Console base
    draw_rectangle(cx - 9.0, cy + 6.0, 18.0, 10.0, frame);
    // Screen
    draw_rectangle(cx - 12.0, cy - 14.0, 24.0, 20.0, frame);
    draw_rectangle(cx - 10.0, cy - 12.0, 20.0, 16.0, screen);

    // Twinkling constellation on the screen
    let pts = [(-6.0, -8.0), (-1.0, -4.0), (4.0, -7.0), (6.0, 0.0), (0.0, 1.0)];
    let star = Color::from_rgba(255, 235, 140, 255);
    for w in pts.windows(2) {
        draw_line(cx + w[0].0, cy + w[0].1, cx + w[1].0, cy + w[1].1, 1.0,
            Color::from_rgba(120, 160, 220, 200));
    }
    for (i, (px, py)) in pts.iter().enumerate() {
        let tw = (time * 3.0 + i as f32).sin() * 0.5 + 0.5;
        draw_circle(cx + px, cy + py, 1.4 + tw * 1.0, star);
    }
}

pub fn draw_old_oak(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0;
    let sway = (time * 0.5).sin() * 1.0;

    // Trunk
    draw_rectangle(cx - 6.0, cy - 4.0, 12.0, 24.0, Color::from_rgba(93, 64, 55, 255));

    // Canopy
    draw_circle(cx + sway, cy - 14.0, 16.0, Color::from_rgba(46, 125, 50, 255));
    draw_circle(cx - 6.0 + sway, cy - 8.0, 10.0, Color::from_rgba(56, 142, 60, 255));
    draw_circle(cx + 6.0 + sway, cy - 8.0, 10.0, Color::from_rgba(56, 142, 60, 255));

    // Face
    let eye = Color::from_rgba(51, 51, 51, 255);
    draw_rectangle(cx - 3.0, cy - 2.0, 2.0, 2.0, eye);
    draw_rectangle(cx + 1.0, cy - 2.0, 2.0, 2.0, eye);
    draw_line(cx - 2.0, cy + 2.0, cx + 2.0, cy + 2.0, 1.0, eye);
}
