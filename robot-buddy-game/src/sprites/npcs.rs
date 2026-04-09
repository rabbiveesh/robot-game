use macroquad::prelude::*;

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
