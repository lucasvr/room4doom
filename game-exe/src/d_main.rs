//! The main loop driver. The primary function is the main loop which attempts to
//! run all tics then dislpay the result. Handling of actual game-exe state is done
//! withing the `Game` object.

use std::error::Error;

use game_state::{Game, GameState};
use gameplay::{
    log::{self, error, info},
    MapObject,
};
use golem::Context;
use input::Input;
use menu_doom::MenuDoom;
use menu_traits::{MenuDraw, MenuResponder, MenuTicker};
use render_soft::SoftwareRenderer;
use render_traits::{PixelBuf, PlayRenderer};
use sdl2::{keyboard::Scancode, rect::Rect, video::Window};
use sound_traits::SoundAction;
use wad::lumps::{WadFlat, WadPatch};

use crate::{
    cheats::Cheats,
    shaders::{basic::Basic, cgwg_crt::Cgwgcrt, lottes_crt::LottesCRT, Drawer, Shaders},
    test_funcs::*,
    timestep::TimeStep,
    CLIOptions,
};

/// Never returns
pub fn d_doom_loop(
    mut game: Game,
    mut input: Input,
    gl: Window,
    ctx: Context,
    options: CLIOptions,
) -> Result<(), Box<dyn Error>> {
    // TODO: implement an openGL or Vulkan renderer
    let mut renderer = SoftwareRenderer::new(
        game.pic_data.clone(),
        matches!(options.verbose, log::LevelFilter::Debug),
    );

    let mut timestep = TimeStep::new();
    let mut render_buffer = PixelBuf::new(320, 200);

    // TODO: sort this block of stuff out
    let wsize = gl.drawable_size();
    let ratio = wsize.1 as f32 * 1.333333;
    let xp = (wsize.0 as f32 - ratio) / 2.0;

    let crop_rect = Rect::new(xp as i32, 0, ratio as u32, wsize.1);

    ctx.set_viewport(
        crop_rect.x as u32,
        crop_rect.y as u32,
        crop_rect.width(),
        crop_rect.height(),
    );

    let mut shader: Box<dyn Drawer> = if let Some(shader) = options.shader {
        match shader {
            Shaders::None => Box::new(Basic::new(&ctx)),
            Shaders::Lottes => Box::new(LottesCRT::new(&ctx)),
            Shaders::Cgwg => Box::new(Cgwgcrt::new(&ctx, crop_rect.width(), crop_rect.height())),
        }
    } else {
        Box::new(Basic::new(&ctx))
    };
    shader.set_tex_filter().unwrap();

    let mut pal_num = 0;
    let mut image_num = 0;
    let mut tex_num = 0;
    let mut flat_num = 0;
    let mut sprite_num = 119;
    let images: Option<Vec<WadPatch>> = if options.image_cycle_test || options.texture_test {
        Some(game.wad_data.patches_iter().collect())
    } else {
        None
    };
    let flats: Option<Vec<WadFlat>> = if options.flats_test {
        Some(game.wad_data.flats_iter().collect())
    } else {
        None
    };
    let sprites: Option<Vec<WadPatch>> = if options.sprites_test {
        let sprites: Vec<WadPatch> = game.wad_data.sprites_iter().collect();
        let image = &sprites[sprite_num];
        info!("{}", image.name);
        Some(sprites)
    } else {
        None
    };

    let mut cheats = Cheats::new();
    let mut menu = MenuDoom::new(&game.wad_data);
    loop {
        if !game.running() {
            break;
        }
        // The game-exe is split in to two parts:
        // - tickers, these update all states (game-exe, menu, hud, automap etc)
        // - drawers, these take a state from above and display it to the user

        // Update the game-exe state
        try_run_tics(&mut game, &mut input, &mut menu, &mut cheats, &mut timestep);

        // Update the positional sounds
        // Update the listener of the sound server. Will always be consoleplayer.
        if let Some(mobj) = game.players[game.consoleplayer].mobj() {
            let uid = mobj as *const MapObject as usize;
            game.snd_command
                .send(SoundAction::UpdateListener {
                    uid,
                    x: mobj.xy.x,
                    y: mobj.xy.y,
                    angle: mobj.angle.rad(),
                })
                .unwrap();
        }

        // Draw everything to the buffer
        d_display(&mut renderer, &mut menu, &game, &mut render_buffer);

        if options.palette_test {
            palette_test(pal_num, &mut game, &mut render_buffer);
        }

        if let Some(name) = options.image_test.clone() {
            image_test(&name.to_ascii_uppercase(), &game, &mut render_buffer);
        }
        if let Some(images) = &images {
            patch_select_test(&images[image_num], &game, &mut render_buffer);
        }
        if let Some(flats) = &flats {
            flat_select_test(&flats[flat_num], &game, &mut render_buffer);
        }
        if let Some(sprites) = &sprites {
            patch_select_test(&sprites[sprite_num], &game, &mut render_buffer);
        }
        if options.texture_test {
            texture_select_test(
                game.pic_data.borrow_mut().get_texture(tex_num),
                &game,
                &mut render_buffer,
            );
        }

        shader.clear();
        shader.set_image_data(render_buffer.read_pixels(), render_buffer.size());
        shader.draw().unwrap();

        gl.gl_swap_window();

        // FPS rate updates every second
        if let Some(_fps) = timestep.frame_rate() {
            //println!("{:?}", fps);

            if options.palette_test {
                if pal_num == 13 {
                    pal_num = 0
                } else {
                    pal_num += 1;
                }
            }

            if let Some(images) = &images {
                image_num += 1;
                if image_num == images.len() {
                    image_num = 0;
                }
            }

            if options.texture_test {
                if tex_num < game.pic_data.borrow_mut().num_textures() - 1 {
                    tex_num += 1;
                } else {
                    tex_num = 0;
                }
            }

            if let Some(flats) = &flats {
                flat_num += 1;
                if flat_num == flats.len() {
                    flat_num = 0;
                }
            }

            if let Some(sprites) = &sprites {
                sprite_num += 1;
                if sprite_num == sprites.len() {
                    sprite_num = 0;
                }
                let image = &sprites[sprite_num];
                info!("{}", image.name);
            }
        }
    }
    Ok(())
}

/// D_Display
/// Does a bunch of stuff in Doom...
fn d_display(
    rend: &mut impl PlayRenderer,
    menu: &mut impl MenuDraw,
    game: &Game,
    pixels: &mut PixelBuf,
) {
    let automap_active = false;
    //if (gamestate == GS_LEVEL && !automapactive && gametic)

    let wipe = if game.game_state != game.wipe_game_state {
        // TODO: wipe_StartScreen(0, 0, SCREENWIDTH, SCREENHEIGHT);
        true
    } else {
        false
    };

    // Drawing order is different for RUST4DOOM as the screensize-statusbar is
    // never taken in to account. A full Doom-style statusbar will never be added
    // instead an "overlay" style bar will be done.
    if game.game_state == GameState::Level && game.game_tic != 0 {
        if !automap_active {
            if let Some(ref level) = game.level {
                if !game.player_in_game[0] {
                    return;
                }
                if game.players[0].mobj().is_none() {
                    error!("Active console player has no MapObject, can't render player view");
                } else {
                    let player = &game.players[game.consoleplayer];
                    rend.render_player_view(player, level, pixels);
                }
            }
        }
        // TODO: HU_Drawer();
        // Fake crosshair
        pixels.set_pixel(320 / 2, 200 / 2, 200, 14, 14, 255);
    }

    match game.game_state {
        GameState::Level => {
            // TODO: Automap draw
            // TODO: Statusbar draw
        }
        GameState::Intermission => {
            // TODO: WI_Drawer();
        }
        GameState::Finale => {
            // TODO: F_Drawer();
        }
        GameState::Demo => {
            // TODO: D_PageDrawer();
        }
        _ => {}
    }

    // // menus go directly to the screen
    menu.render_menu(pixels); // menu is drawn even on top of everything
                              // net update does i/o and buildcmds...
                              // TODO: NetUpdate(); // send out any new accumulation
}

fn try_run_tics<M>(
    game: &mut Game,
    input: &mut Input,
    menu: &mut M,
    cheats: &mut Cheats,
    timestep: &mut TimeStep,
) where
    M: MenuResponder + MenuTicker,
{
    // TODO: net.c starts here
    process_events(game, input, menu, cheats); // D_ProcessEvents

    // Build tics here?
    timestep.run_this(|_| {
        // G_Ticker
        game.ticker();
        menu.ticker(game);
        game.game_tic += 1;
    });
}

fn process_events(
    game: &mut Game,
    input: &mut Input,
    menu: &mut impl MenuResponder,
    cheats: &mut Cheats,
) {
    // required for cheats and menu so they don't receive multiple key-press fo same key
    let callback = |sc: Scancode| {
        if game.level.is_some() {
            cheats.check_input(sc, game);
        }

        menu.responder(sc, game)
    };
    if !input.update(callback) {
        let console_player = game.consoleplayer;
        // net update does i/o and buildcmds...
        // TODO: NetUpdate(); // send out any new accumulation

        // TODO: Network code would update each player slot with incoming TicCmds...
        let cmd = input.events.build_tic_cmd(&input.config);
        game.netcmds[console_player][0] = cmd;
    }
}
