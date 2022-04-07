//! Doom source name `p_enemy`

use log::error;

use super::{
    mobj::{MapObject, MapObjectFlag},
    utilities::point_to_angle_2,
};

/// A_FaceTarget
pub fn a_facetarget(actor: &mut MapObject) {
    actor.flags &= !(MapObjectFlag::Ambush as u32);

    if let Some(target) = actor.target {
        unsafe {
            let angle = point_to_angle_2(&actor.xy, &(*target).xy);
            actor.angle = angle;

            if (*target).flags & MapObjectFlag::Shadow as u32 == MapObjectFlag::Shadow as u32 {
                // TODO: actor.angle += P_SubRandom() << 21;
            }
        }
    }
}

/// Actor has a melee attack,
/// so it tries to close as fast as possible
pub fn a_chase(actor: &mut MapObject) {
    if actor.reactiontime > 0 {
        actor.reactiontime -= 1;
    }

    // modify target threshold
    if actor.threshold > 0 {
        if
        // TODO: gameversion > exe_doom_1_2 &&
        actor.target.is_none() || actor.target.is_some() {
            // unsafe { actor.target.as_ref().unwrap().health <= 0 }
            if let Some(target) = actor.target {
                unsafe {
                    if (*target).health <= 0 {
                        actor.threshold = 0;
                    }
                }
            }
        } else {
            actor.threshold -= 1;
        }
    }

    actor.set_state(actor.info.spawnstate);
    //

    //
    // // turn towards movement direction if not there yet
    // if (actor->movedir < 8)
    // {
    // actor->angle &= (7 << 29);
    // delta = actor->angle - (actor->movedir << 29);
    //
    // if (delta > 0)
    // actor->angle -= ANG90 / 2;
    // else if (delta < 0)
    // actor->angle += ANG90 / 2;
    // }
    //
    // if (!actor->target || !(actor->target->flags & SHOOTABLE))
    // {
    // // look for a new target
    // if (P_LookForPlayers(actor, true))
    // return; // got a new target
    //
    // P_SetMobjState(actor, actor->info->spawnstate);
    // return;
    // }
    //
    // // do not attack twice in a row
    // if (actor->flags & JUSTATTACKED)
    // {
    // actor->flags &= ~JUSTATTACKED;
    // if (gameskill != sk_nightmare && !fastparm)
    // P_NewChaseDir(actor);
    // return;
    // }
    //
    // // check for melee attack
    // if (actor->info->meleestate && P_CheckMeleeRange(actor))
    // {
    // if (actor->info->attacksound)
    // S_StartSound(actor, actor->info->attacksound);
    //
    // P_SetMobjState(actor, actor->info->meleestate);
    // return;
    // }
    //
    // // check for missile attack
    // if (actor->info->missilestate)
    // {
    // if (gameskill < sk_nightmare && !fastparm && actor->movecount)
    // {
    // goto nomissile;
    // }
    //
    // if (!P_CheckMissileRange(actor))
    // goto nomissile;
    //
    // P_SetMobjState(actor, actor->info->missilestate);
    // actor->flags |= JUSTATTACKED;
    // return;
    // }
    //
    // // ?
    // nomissile:
    // // possibly choose another target
    // if (netgame && !actor->threshold && !P_CheckSight(actor, actor->target))
    // {
    // if (P_LookForPlayers(actor, true))
    // return; // got a new target
    // }
    //
    // // chase towards player
    // if (--actor->movecount < 0 || !P_Move(actor))
    // {
    // P_NewChaseDir(actor);
    // }
    //
    // // make active sound
    // if (actor->info->activesound && P_Random() < 3)
    // {
    // S_StartSound(actor, actor->info->activesound);
    // }
}

/// Stay in state until a player is sighted.
pub fn a_look(actor: &mut MapObject) {
    //error!("a_look not implemented");
    // mobj_t *targ;
    //
    // actor->threshold = 0; // any shot will wake up
    // targ = actor->subsector->sector->soundtarget;
    //
    // if (targ && (targ->flags & SHOOTABLE))
    // {
    // actor->target = targ;
    //
    // if (actor->flags & AMBUSH)
    // {
    // if (P_CheckSight(actor, actor->target))
    // goto seeyou;
    // }
    // else
    // goto seeyou;
    // }
    //
    // if (!P_LookForPlayers(actor, false))
    // return;
    //
    // // go into chase state
    // seeyou:
    // if (actor->info->seesound)
    // {
    // int sound;
    //
    // switch (actor->info->seesound)
    // {
    // case sfx_posit1:
    // case sfx_posit2:
    // case sfx_posit3:
    // sound = sfx_posit1 + P_Random() % 3;
    // break;
    //
    // case sfx_bgsit1:
    // case sfx_bgsit2:
    // sound = sfx_bgsit1 + P_Random() % 2;
    // break;
    //
    // default:
    // sound = actor->info->seesound;
    // break;
    // }
    //
    // if (actor->type == MT_SPIDER || actor->type == MT_CYBORG)
    // {
    // // full volume
    // S_StartSound(NULL, sound);
    // }
    // else
    // S_StartSound(actor, sound);
    // }
    //
    // P_SetMobjState(actor, actor->info->seestate);
    // actor.set_state(actor.info.seestate);
}

pub fn a_fire(_actor: &mut MapObject) {
    error!("a_fire not implemented");
    // mobj_t *dest;
    // mobj_t *target;
    // unsigned an;
    //
    // dest = actor->tracer;
    // if (!dest)
    // return;
    //
    // target = P_SubstNullMobj(actor->target);
    //
    // // don't move it if the vile lost sight
    // if (!P_CheckSight(target, dest))
    // return;
    //
    // an = dest->angle >> ANGLETOFINESHIFT;
    //
    // P_UnsetThingPosition(actor);
    // actor->x = dest->x + FixedMul(24 * FRACUNIT, finecosine[an]);
    // actor->y = dest->y + FixedMul(24 * FRACUNIT, finesine[an]);
    // actor->z = dest->z;
    // P_SetThingPosition(actor);
}

pub fn a_scream(_actor: &mut MapObject) {
    error!("a_scream not implemented");
    // int sound;
    //
    // switch (actor->info->deathsound)
    // {
    // case 0:
    // return;
    //
    // case sfx_podth1:
    // case sfx_podth2:
    // case sfx_podth3:
    // sound = sfx_podth1 + P_Random() % 3;
    // break;
    //
    // case sfx_bgdth1:
    // case sfx_bgdth2:
    // sound = sfx_bgdth1 + P_Random() % 2;
    // break;
    //
    // default:
    // sound = actor->info->deathsound;
    // break;
    // }
    //
    // // Check for bosses.
    // if (actor->type == MT_SPIDER || actor->type == MT_CYBORG)
    // {
    // // full volume
    // S_StartSound(NULL, sound);
    // }
    // else
    // S_StartSound(actor, sound);
}

pub fn a_fall(actor: &mut MapObject) {
    // actor is on ground, it can be walked over
    actor.flags &= !(MapObjectFlag::Solid as u32);

    // So change this if corpse objects
    // are meant to be obstacles.
}

pub fn a_explode(actor: &mut MapObject) {
    println!("BOOOOOOM!");
    dbg!(actor.target.is_some());
    actor.radius_attack(128.0);
}

pub fn a_xscream(_actor: &mut MapObject) {
    error!("a_xscream not implemented");
    // if (actor->info->painsound)
    // S_StartSound(actor, actor->info->painsound);
}

pub fn a_keendie(_actor: &mut MapObject) {
    error!("a_keendie not implemented");
}

pub fn a_hoof(actor: &mut MapObject) {
    error!("a_hoof not implemented");
}

pub fn a_metal(actor: &mut MapObject) {
    error!("a_metal not implemented");
}

pub fn a_babymetal(actor: &mut MapObject) {
    error!("a_babymetal not implemented");
}

pub fn a_brainawake(actor: &mut MapObject) {
    error!("a_brainawake not implemented");
}

pub fn a_braindie(actor: &mut MapObject) {
    error!("a_braindie not implemented");
}

pub fn a_brainspit(actor: &mut MapObject) {
    error!("a_brainspit not implemented");
}

pub fn a_brainpain(actor: &mut MapObject) {
    error!("a_brainpain not implemented");
}

pub fn a_brainscream(actor: &mut MapObject) {
    error!("a_brainscream not implemented");
}

pub fn a_brainexplode(actor: &mut MapObject) {
    error!("a_brainexplode not implemented");
}

pub fn a_spawnfly(actor: &mut MapObject) {
    error!("a_spawnfly not implemented");
}

pub fn a_spawnsound(actor: &mut MapObject) {
    error!("a_spawnsound not implemented");
}

pub fn a_vilestart(actor: &mut MapObject) {
    error!("a_vilestart not implemented");
}

pub fn a_vilechase(actor: &mut MapObject) {
    error!("a_vilechase not implemented");
}

pub fn a_viletarget(actor: &mut MapObject) {
    error!("a_viletarget not implemented");
}

pub fn a_vileattack(actor: &mut MapObject) {
    error!("a_vileattack not implemented");
}

pub fn a_posattack(actor: &mut MapObject) {
    error!("a_posattack not implemented");
}

pub fn a_sposattack(actor: &mut MapObject) {
    error!("a_sposattack not implemented");
}

pub fn a_cposattack(actor: &mut MapObject) {
    error!("a_cposattack not implemented");
}

pub fn a_bspiattack(actor: &mut MapObject) {
    error!("a_bspiattack not implemented");
}

pub fn a_skullattack(actor: &mut MapObject) {
    error!("a_skullattack not implemented");
}

pub fn a_headattack(actor: &mut MapObject) {
    error!("a_headattack not implemented");
}

pub fn a_sargattack(actor: &mut MapObject) {
    error!("a_sargattack not implemented");
}

pub fn a_bruisattack(actor: &mut MapObject) {
    error!("a_bruisattack not implemented");
}

pub fn a_cposrefire(actor: &mut MapObject) {
    error!("a_cposrefire not implemented");
}

pub fn a_cyberattack(actor: &mut MapObject) {
    error!("a_cyberattack not implemented");
}

pub fn a_troopattack(actor: &mut MapObject) {
    error!("a_troopattack not implemented");
}

pub fn a_pain(actor: &mut MapObject) {
    error!("a_pain not implemented");
    // if (actor->info->painsound)
    // S_StartSound(actor, actor->info->painsound);
}

pub fn a_painattack(actor: &mut MapObject) {
    error!("a_painattack not implemented");
}

pub fn a_paindie(actor: &mut MapObject) {
    error!("a_paindie not implemented");
}

pub fn a_fatattack1(actor: &mut MapObject) {
    error!("a_fatattack1 not implemented");
}
pub fn a_fatattack2(actor: &mut MapObject) {
    error!("a_fatattack2 not implemented");
}
pub fn a_fatattack3(actor: &mut MapObject) {
    error!("a_fatattack3 not implemented");
}

pub fn a_fatraise(actor: &mut MapObject) {
    error!("a_fatraise not implemented");
}

pub fn a_spidrefire(actor: &mut MapObject) {
    error!("a_spidrefire not implemented");
}

pub fn a_bossdeath(actor: &mut MapObject) {
    error!("a_bossdeath not implemented");
}

pub fn a_skelwhoosh(actor: &mut MapObject) {
    error!("a_skelwhoosh not implemented");
}

pub fn a_skelfist(actor: &mut MapObject) {
    error!("a_skelfist not implemented");
}

pub fn a_skelmissile(actor: &mut MapObject) {
    error!("a_skelmissile not implemented");
}

pub fn a_tracer(actor: &mut MapObject) {
    error!("a_tracer not implemented");
}

pub fn a_startfire(actor: &mut MapObject) {
    error!("a_startfire not implemented");
}

pub fn a_firecrackle(actor: &mut MapObject) {
    error!("a_firecrackle not implemented");
}

pub fn a_playerscream(actor: &mut MapObject) {
    error!("a_playerscream not implemented");
}
