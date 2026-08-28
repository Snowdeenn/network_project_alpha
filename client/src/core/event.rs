pub fn handle_event(
    event: utils::protocol::GameEvent,
    app_resource: &mut crate::app::resources::Resources,
) {
    match event.kind {
        utils::protocol::GameEventKind::ShopOpened { inventory } => {
            let mut shop_ui = app_resource.write_resource::<crate::core::shop_state::ShopUiState>();
            shop_ui.open(inventory);
        }
        utils::protocol::GameEventKind::WaveEnd { time_between_wave } => {
            let mut phase = app_resource.write_resource::<crate::core::game_phase::GamePhase>();
            *phase = crate::core::game_phase::GamePhase::BetweenWave {
                time_remaining: time_between_wave,
                shop_available: true,
            };
        }
        utils::protocol::GameEventKind::WaveStart { .. } => {
            {
                let mut phase = app_resource.write_resource::<crate::core::game_phase::GamePhase>();
                *phase = crate::core::state_logic::game_phase::GamePhase::Wave
            }
            app_resource
                .write_resource::<crate::core::shop_state::ShopUiState>()
                .close();
        }
        utils::protocol::GameEventKind::BossSpawn { .. } => {}
        utils::protocol::GameEventKind::PlayerDied { .. } => {
            {
                tracing::info!("mort reçu");
                let mut phase = app_resource.write_resource::<crate::core::game_phase::GamePhase>();
                *phase = crate::core::state_logic::game_phase::GamePhase::Dead;
            }
            {
                let mut ui_state =
                    app_resource.write_resource::<crate::core::ui_state::UiState>();
                ui_state.spectator_mode = Some(crate::core::ui_state::SpectatorMode::Free);
            }
        }
        utils::protocol::GameEventKind::ItemBought { slot } => {
            app_resource
                .write_resource::<crate::core::shop_state::ShopUiState>()
                .item_bought(slot as usize);
        }
        utils::protocol::GameEventKind::PurchaseFailed { slot } => {
            app_resource
                .write_resource::<crate::core::shop_state::ShopUiState>()
                .purchase_failed(slot as usize);
        }
        utils::protocol::GameEventKind::SpawnRect {
            x,
            y,
            half_length,
            half_width,
            dir,
        } => {
            app_resource
                .write_resource::<crate::core::debug_state::DebugState>()
                .add_rect(x, y, half_length, half_width, dir);
        }
        utils::protocol::GameEventKind::EntityHit { pos } => {
            app_resource
                .write_resource::<crate::core::debug_state::DebugState>()
                .set_hit_anim(pos);
        }
        utils::protocol::GameEventKind::DebugCollider { x, y } => {
            let mut debug = app_resource.write_resource::<crate::core::debug_state::DebugState>();
            if !debug.cleared {
                debug.collider.clear();
                debug.cleared = true;
            }
            debug.add_collider(x, y);
        }
        utils::protocol::GameEventKind::SharedLivesUpdate { remaining, max } => {
            let mut ui = app_resource.write_resource::<crate::core::ui_state::UiState>();
            println!("SharedLivesUpdate reçu");
            ui.shared_lives.current = remaining;
            ui.shared_lives.max = max;
        }
        utils::protocol::GameEventKind::RespawnScheduled {  delay_secs, .. } => {
            let mut ui_state = app_resource.write_resource::<crate::core::ui_state::UiState>();
            ui_state.respawn_timer = Some(delay_secs);
        }
        utils::protocol::GameEventKind::GameOver => {}
        utils::protocol::GameEventKind::PlayerHit => (), // On gère ça sur le niveau au dessus
        utils::protocol::GameEventKind::PlayerSpawn {
            client_id,
            entity_id,
        } => {
            println!("player spawn reçu client_id: {client_id}, entity_id: {entity_id}");
            let mut local_id = app_resource.write_resource::<crate::core::LocalId>();
            local_id.entity_id = entity_id;
            local_id.client_id = client_id;
        }
        // Event qu'on utilise pour demander au server le respawn du joueur
        utils::protocol::GameEventKind::RequestRespawn { .. } => (),
        // Event qu'on utilise en interne du server pour respawn le joueur
        utils::protocol::GameEventKind::RespawnError { .. } => {
            *app_resource.write_resource::<crate::core::game_phase::GamePhase>() = crate::core::game_phase::GamePhase::Dead;
        },
        utils::protocol::GameEventKind::RespawnPlayer { .. } => (),
        utils::protocol::GameEventKind::RespawnAccept { .. } => {
            let mut game_phase = app_resource.write_resource::<crate::core::game_phase::GamePhase>();
            *game_phase = crate::core::game_phase::GamePhase::Wave;
        }
    }
}

pub fn handle_shop_ui_event(
    event: &utils::protocol::GameEvent,
    ui_ctx: &mut nodus::UiContext,
    shop_ids: &utils::ids::Register,
) {
    let root = match shop_ids.get::<nodus::NodeId>(crate::key::shop::ROOT) {
        Some(id) => id,
        None => {
            tracing::warn!("L'id {} est absent du register", crate::key::shop::ROOT);
            return;
        }
    };
    match &event.kind {
        utils::protocol::GameEventKind::ShopOpened { inventory } => {
            // afficher le shop
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: root,
                visible: true,
            });

            // mettre à jour les 3 cartes
            for (slot, item_opt) in inventory.iter().enumerate() {
                let card = match shop_ids
                    .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[slot])
                {
                    Some(id) => id,
                    None => {
                        tracing::warn!(
                            "L'id {} est absent du register",
                            crate::key::shop::SHOP_CARD_KEYS[slot]
                        );
                        return;
                    }
                };
                if let Some(item) = item_opt {
                    let border_color = match item.effect_type {
                        utils::protocol::EffectType::Health => utils::colors::Color::DARKGREEN,
                        utils::protocol::EffectType::Damage => utils::colors::Color::MAROON,
                        utils::protocol::EffectType::Speed => utils::colors::Color::DARKBLUE,
                        utils::protocol::EffectType::Gold => utils::colors::Color::GOLD,
                    };
                    ui_ctx.send_event(nodus::UIEvent::SetColor {
                        target: card.root,
                        color: border_color,
                    });
                }
                match item_opt {
                    Some(item) => {
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.name,
                            content: item.name.clone(),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.desc,
                            content: item.description.clone(),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetText {
                            target: card.price,
                            content: format!("PRIX: {} OR", item.price),
                        });
                        ui_ctx.send_event(nodus::UIEvent::SetVisible {
                            target: card.sold_overlay,
                            visible: false,
                        });
                    }
                    None => {
                        ui_ctx.send_event(nodus::UIEvent::SetVisible {
                            target: card.sold_overlay,
                            visible: true,
                        });
                    }
                }
            }
        }

        utils::protocol::GameEventKind::WaveStart { .. } => {
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: root,
                visible: false,
            });
        }

        utils::protocol::GameEventKind::ItemBought { slot } => {
            let card = match shop_ids
                .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[*slot])
            {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "L'id {} est absent du register",
                        crate::key::shop::SHOP_CARD_KEYS[*slot]
                    );
                    return;
                }
            };
            // tween fade sur sold_overlay
            ui_ctx.tween.add(nodus::Tween {
                target: card.sold_overlay,
                property: nodus::TweenProperty::Opacity { from: 0.0, to: 1.0 },
                duration: crate::core::config::SOLD_ANIM_DURATION,
                elapsed: 0.0,
                easing: nodus::easing::ease_in_out_quad,
                done: false,
                on_complete: vec![
                    nodus::UIEvent::SetColor {
                        target: card.sold_overlay,
                        color: utils::colors::Color::new(40, 40, 40, 255),
                    },
                    nodus::UIEvent::SetOpacity {
                        target: card.sold_overlay,
                        opacity: 1.0,
                    },
                    nodus::UIEvent::SetText {
                        target: card.sold_text,
                        content: "VENDU".to_string(),
                    },
                    nodus::UIEvent::SetVisible {
                        target: card.sold_text,
                        visible: true,
                    },
                ],
            });
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: card.sold_overlay,
                visible: true,
            });
        }

        utils::protocol::GameEventKind::PurchaseFailed { slot } => {
            let card = match shop_ids
                .get::<crate::ui::hud::ShopCardIds>(crate::key::shop::SHOP_CARD_KEYS[*slot])
            {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        "L'id {} est absent du register",
                        crate::key::shop::SHOP_CARD_KEYS[*slot]
                    );
                    return;
                }
            };
            // tween flash rouge
            ui_ctx.tween.add(nodus::Tween {
                target: card.error_overlay,
                property: nodus::TweenProperty::Opacity { from: 0.7, to: 0.0 },
                duration: 1.5,
                elapsed: 0.0,
                easing: nodus::easing::ease_out_quad,
                done: false,
                on_complete: vec![nodus::UIEvent::SetVisible {
                    target: card.error_overlay,
                    visible: false,
                }],
            });
            ui_ctx.send_event(nodus::UIEvent::SetVisible {
                target: card.error_overlay,
                visible: true,
            });
            ui_ctx.send_event(nodus::UIEvent::SetOpacity {
                target: card.error_overlay,
                opacity: 0.7,
            });
        }
        utils::protocol::GameEventKind::WaveEnd { .. } => {}
        _ => {}
    }
}
