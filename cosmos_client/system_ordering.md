# Cosmos Client — System Ordering

```mermaid
graph TD
    %% ==================== STARTUP / STATE TRANSITIONS ====================
    subgraph STARTUP ["Startup / State Transitions"]
        direction TB

        subgraph SS_START ["Startup Schedule"]
            SS1["setup (skybox)<br/><i>skybox/mod.rs</i>"]
            SS2["init_input<br/><i>input/inputs.rs</i>"]
            SS3["setup_window<br/><i>window/setup.rs</i>"]
            SS4["init_default_font<br/><i>ui/font.rs</i>"]
            SS5["remove_gravity<br/><i>physics/mod.rs</i>"]
            SS6["register_biospheres<br/><i>structure/planet/biosphere.rs</i>"]
            SS7["create_mining_laser_mesh<br/><i>structure/systems/mining_laser_system.rs</i>"]
            SS8["setup (menu panorama, photo booth)<br/><i>ui/main_menu/ + ui/item_renderer/</i>"]
        end

        subgraph SS_PRELOAD ["OnEnter: PreLoading"]
            PL1["init_settings_lang<br/><i>settings/mod.rs</i>"]
            PL2["register_settings<br/><i>settings/mod.rs</i>"]
            PL3["create_railgun_mesh<br/><i>structure/systems/railgun_system.rs</i>"]
        end

        subgraph SS_LOAD ["OnEnter: Loading"]
            L1["load_assets (AudioSources ×9)<br/><i>audio/music/ + structure/systems/</i>"]
            L2["load_assets (Images ×2)<br/><i>ui/crosshair.rs + ui/friends/</i>"]
            L3["register_meshes<br/><i>rendering/mod.rs</i>"]
            L4["create_laser_mesh, create_missile_mesh<br/><i>projectiles/</i>"]
            L5["load_default_songs<br/><i>audio/music/dynamic_music.rs</i>"]
            L6["load_settings<br/><i>settings/mod.rs</i>"]
        end

        subgraph SS_POSTLOAD ["OnEnter / OnExit: PostLoading"]
            PS1["OnEnter: setup_textures<br/><i>asset/asset_loading.rs</i>"]
            PS2["OnEnter: load_descriptions<br/><i>item/descriptions.rs</i>"]
            PS3["OnExit: [ItemMeshingLoadingSet] (.chain)<br/>LoadItemRenderingInfo→LoadBlockModels→LoadItemModels→GenerateMeshes<br/><i>asset/asset_loading.rs + rendering/ + item/</i>"]
            PS4["OnExit: register_materials<br/><i>asset/materials/mod.rs</i>"]
            PS5["OnExit: register_block_meshes<br/><i>rendering/mod.rs</i>"]
        end

        subgraph SS_PLAY ["OnEnter / OnExit: Playing"]
            PY1["setup_chat_display, setup_chat_box<br/><i>chat/mod.rs</i>"]
            PY2["add_hotbar, add_item_text<br/><i>ui/hotbar.rs</i>"]
            PY3["add_crosshair, add_text (debug)<br/><i>ui/crosshair.rs + ui/debug_info_display.rs</i>"]
            PY4["setup_camera→create_focused_ui (.chain)<br/><i>ui/focus_cam/mod.rs</i>"]
            PY5["create_credits_node, init_error_list<br/><i>ui/hud/</i>"]
            PY6["spawn_planet_skysphere<br/><i>structure/planet/planet_skybox.rs</i>"]
            PY7["send_permutation_table_to_worker<br/><i>structure/planet/generation.rs</i>"]
            PY8["create_debug (perf UI)<br/><i>window/setup.rs</i>"]
        end
    end

    %% ==================== FIXEDUPDATE PIPELINE ====================
    subgraph FU ["FIXEDUPDATE — 60 Hz (.chain)"]

        subgraph FU_NETTYRECV ["NettyReceive"]
            direction LR
            NR_RECV["ReceiveMessages"]
            NR_PROC["ProcessReceivedMessages"]
            NR_RECV --> NR_PROC
            NR1["client_sync_players→lerp_towards<br/><i>netty/gameplay/receiver.rs</i>"]
            NR2["lasers_netty, shop_listen_netty<br/><i>projectiles/lasers.rs + shop/netty.rs</i>"]
            NR3["receive_asteroids<br/><i>structure/asteroid/sync.rs</i>"]
            NR4["setup_lod_generation<br/><i>structure/planet/generation.rs</i>"]
            NR5["(ComponentSyncingSet) client_deserialize_parent<br/>→client_remove_parent<br/><i>netty/sync/component/mod.rs</i>"]
        end

        subgraph FU_MAIN ["Main"]
            subgraph MAIN_IN ["InputProcessing"]
                MI1["adjust_player_to_face_camera<br/><i>camera/camera_controller.rs</i>"]
                MI2["add_alignment→process_player_movement (.chain)<br/><i>entities/player/player_movement.rs</i>"]
                MI3["on_use_blueprint<br/><i>item/usable/blueprint.rs</i>"]
                MI4["process_ship_movement + reset_cursor<br/><i>structure/ship/ship_movement.rs</i>"]
            end
            subgraph MAIN_SIM ["Simulation"]
                MS1["align_player→align_on_ship (.chain)<br/><i>structure/planet/align_player.rs</i>"]
                MS2["add_last_planet_rotation→<br/>rotate_client_around_planets (.chain)<br/><i>structure/planet/rotate_around_planet.rs</i>"]
            end
            subgraph MAIN_EV ["EventProcessing"]
                ME1["sync_recipes (basic + advanced)<br/><i>crafting/recipes/</i>"]
                ME2["on_add_player<br/><i>entities/player/mod.rs</i>"]
                ME3["on_teleport<br/><i>entities/player/teleport.rs</i>"]
                ME4["on_respawn<br/><i>entities/player/death.rs</i>"]
            end
            subgraph MAIN_LT ["Late"]
                ML1["on_quest_complete, display_active_mission<br/><i>quest/ui/hud.rs</i>"]
                ML2["play_warp_sound, on_shutdown_warp<br/><i>structure/systems/warp/warp_drive.rs</i>"]
            end
            MAIN_IN --> MAIN_SIM --> MAIN_EV --> MAIN_LT
        end

        subgraph FU_LOCSYNC ["LocationSyncing"]
        end

        subgraph FU_PREPHYS ["PrePhysics"]
            PP1["disable_colliders<br/><i>physics/collider_disabling.rs</i>"]
        end

        subgraph FU_RAPIER ["Rapier Physics"]
            RP1["SyncBackend→StepSimulation→Writeback"]
        end

        subgraph FU_POSTPHYS ["PostPhysics"]
            POP1["add_looking_at→compute_looking_at→<br/>process_player_interaction (.chain)<br/><i>interactions/block_interactions.rs</i>"]
            POP2["respond_to_collisions→<br/>remove_parent_when_too_far (.chain)<br/><i>structure/ship/mod.rs</i>"]
            POP3["append_grounded_check→<br/>check_grounded (.chain)<br/><i>entities/player/player_movement.rs</i>"]
            POP4["on_look_at_interactable_block<br/><i>ui/hud/interactable_block.rs</i>"]
            POP5["control_build_mode<br/><i>structure/shared/build_mode/mod.rs</i>"]
        end

        subgraph FU_LOCSYNC2 ["LocationSyncingPostPhysics"]
            LS2_1["load_planet_chunks→<br/>unload_chunks_far_from_players (.chain)<br/><i>structure/planet/mod.rs</i>"]
        end

        subgraph FU_POSTLOC ["PostLocationSyncingPostPhysics"]
        end

        subgraph FU_NETTYSEND ["NettySend"]
            NS_SF["ComponentSyncingSet::PreComponentSyncing→<br/>DoComponentSyncing→PostComponentSyncing"]
            NS1["send_position<br/><i>netty/gameplay/sync/sync_player.rs</i>"]
            NS2["sync (inventory)<br/><i>inventory/netty.rs</i>"]
            NS3["request_chunks→populate_structures (.chain)<br/><i>structure/chunk_retreiver.rs</i>"]
            NS4["sync structure systems (per type)<br/><i>structure/systems/sync.rs</i>"]
        end

        FU_NETTYRECV --> FU_MAIN --> FU_LOCSYNC --> FU_PREPHYS --> FU_RAPIER --> FU_POSTPHYS --> FU_LOCSYNC2 --> FU_POSTLOC --> FU_NETTYSEND
    end

    %% ==================== UNCONSTRAINED FIXEDUPDATE ====================
    subgraph FU_UNC ["Unconstrained FixedUpdate"]

        subgraph FU_BLOCKMSGS ["Block Event Processing"]
            BM1["handle_block_break/place/interact<br/>show_errors<br/><i>events/block/block_events.rs</i>"]
        end

        subgraph FU_STRUC ["Structure Systems"]
            ST1["replication_listen_netty (structure systems)<br/><i>structure/systems/sync.rs</i>"]
            ST2["client_on_add_ship<br/><i>structure/ship/client_ship_builder.rs</i>"]
        end

        subgraph FU_PROJ ["Projectiles"]
            PJ1["respond_to_explosion, track_time_alive<br/><i>projectiles/missile.rs</i>"]
        end

        subgraph FU_LASERS ["Mining Lasers"]
            LZ1["apply_mining_effects<br/><i>structure/systems/mining_laser_system.rs</i>"]
            LZ2["resize/rotate/remove_dead beams<br/><i>structure/systems/mining_laser_system.rs</i>"]
        end

        subgraph FU_MISC ["Miscellaneous"]
            FM1["render_physical_item<br/><i>item/physical_item.rs</i>"]
            FM2["on_enter_build_mode, change_visuals<br/><i>structure/shared/build_mode/mod.rs</i>"]
            FM3["unload_far_entities (.after LocSyncPostPhys, .before NettySend)<br/><i>loading/mod.rs</i>"]
            FM4["add_render_flag (numeric display)<br/><i>block/blocks/numeric_display.rs</i>"]
        end
    end

    %% ==================== UPDATE SCHEDULE ====================
    subgraph UPDATE ["UPDATE Schedule"]

        subgraph U_NETTY ["Networking"]
            UN1["insert_last_rotation→update_crosshair (.chain)<br/><i>netty/gameplay/receiver.rs</i>"]
            UN2["sync (inventory), receive_asteroids<br/><i>inventory/netty.rs + structure/asteroid/sync.rs</i>"]
        end

        subgraph U_CURSOR ["Cursor / Window"]
            UC1["update_mouse_deltas→<br/>window_focus_changed (.chain)<br/><i>window/setup.rs</i>"]
            UC2["apply_cursor_flags_on_change<br/><i>window/setup.rs</i>"]
            UC3["on_toggle (fullscreen)<br/><i>window/fullscreen_toggle.rs</i>"]
            UC4["show_cursor (main menu)<br/><i>window/setup.rs</i>"]
        end

        subgraph U_ASSETSLOADING ["AssetsSet::AssetsLoading (.chain)"]
            AL1["Check assets ready → assets_done_loading<br/><i>asset/asset_loading.rs</i>"]
            AL2["create_materials (main + LOD)<br/><i>asset/materials/material_types/</i>"]
        end

        subgraph U_ASSETSREADY ["AssetsSet::AssetsReady"]
            AR1["respond_to_remove/add_materials (main + LOD)<br/><i>asset/materials/material_types/</i>"]
        end

        subgraph U_RENDER ["Structure Rendering"]
            UR1["MonitorBlockUpdates<br/><i>rendering/structure_renderer/</i>"]
            UR2["BeginRendering: monitor_needs_rendered→<br/>poll_rendering_chunks (.chain)<br/><i>rendering/structure_renderer/</i>"]
            UR3["CustomRendering"]
            UR1 --> UR2 --> UR3
        end

        subgraph U_UI ["UiSystemSet (.chain: PreDoUi→DoUi→FinishUi)"]
            UU_PRE["PreDoUi"]
            UU_PRE_S1["clear_focus<br/><i>ui/components/mod.rs</i>"]
            UU_PRE_S2["toggle_inventory→close_button→<br/>draw_held_item→on_change_search (.chain)<br/><i>inventory/mod.rs</i>"]
            UU_PRE_S3["open_dye_ui, display_death_ui<br/><i>block/blocks/dye_machine.rs + entities/player/death.rs</i>"]
            UU_PRE_S4["create_ui (reactor)<br/><i>block/multiblocks/reactor/ui.rs</i>"]

            UU_DO["DoUi"]
            UU_DO_S1["RenderItemSystemSet::RenderItems<br/><i>ui/item_renderer/mod.rs</i>"]
            UU_DO_S2["[FabricatorMenuSet]</br>open_menu + populate_menu (.chain)<br/><i>crafting/blocks/*/ui.rs</i>"]
            UU_DO_S3["[ScrollBoxUiSystemSet]<br/>on_add→on_interact→update_scrollbox"]
            UU_DO_S4["[SliderUiSystemSet]<br/>on_add→on_interact→on_change_value"]
            UU_DO_S5["[ButtonUiSystemSet]<br/>on_add→on_change→send_messages"]
            UU_DO_S6["[CheckboxUiSystemSet]<br/>on_add_checkbox→on_change_checkbox (.chain)"]
            UU_DO_S7["[TextInputUiSystemSet]<br/>on_add→on_change→on_clear (.chain)"]
            UU_DO_S8["[ModalUiSystemSet]<br/>on_add_modal, on_add_confirm_modal, on_add_text_modal"]
            UU_DO_S9["[UiWindowSystemSet + UiTabViewSystemSet]"]
            UU_DO_S10["create_settings_screen<br/><i>ui/settings/mod.rs</i>"]
            UU_DO_S11["color_fabricate_button<br/><i>crafting/blocks/advanced_fabricator/ui.rs</i>"]

            UU_FIN["FinishUi"]
            UU_FIN_S1["show_cursor<br/><i>ui/components/show_cursor.rs</i>"]

            UU_PRE --> UU_DO --> UU_FIN
        end

        subgraph U_UI_UNSET ["UiSystemSet — unassigned subset"]
            US1["display_hud_messages<br/><i>ui/message.rs</i>"]
            US2["on_hide_ui→on_change_reasons (.chain)<br/><i>ui/hide.rs</i>"]
            US3["toggle_pause_menu<br/><i>ui/pause/mod.rs</i>"]
            US4["toggle_menu (master menu), toggle_view (focus cam)<br/><i>ui/master_menu/ + ui/focus_cam/</i>"]
            US5["[InventorySet chain (.before PreDoUi)]<br/>toggle_inventory→close→draw_held→search (.chain)→<br/>drop_item (.run_if no_open_menus)→update→<br/>handle_interactions→follow_cursor→toggle_rendering<br/><i>inventory/mod.rs</i>"]
            US6["[ShopLogicSet]</br>open_shop_ui→on_change_mode→on_change_selected→<br/>update_total→update_search→render→<br/>enable_buy→enable_sell<br/><i>shop/ui.rs</i>"]
            US7["[SystemSelectionSet] (hotbar)<br/>add_hotbar_contents→sync_hotbar→populate→<br/>listen_for_changes→monitor_cooldown→<br/>listen_button_presses→tick_text_alpha<br/><i>ui/hotbar.rs</i>"]
            US8["[SystemUsageSet]</br>check_system_in_use→check_removed_pilot (.chain)<br/>check_became_pilot<br/><i>structure/systems/player_interactions.rs</i>"]
            US9["[Chat] on_cycle_chat→display→send→toggle→<br/>fade→remove_old→toggle_visibility (.chain)<br/><i>chat/mod.rs</i>"]
            US10["[COMS] on_remove_coms→on_change→<br/>on_change_selected→on_not_pilot→<br/>on_close_menu (.chain)<br/><i>coms/ui/main_ui.rs</i>"]
            US11["[Tooltip] on_change_looking_at→<br/>add_tooltip_text→on_finish_tooltip<br/><i>ui/hud/tooltip.rs</i>"]
            US12["[Crosshair] update_cursor_pos→<br/>on_change_crosshair_state (.chain)<br/><i>ui/crosshair.rs</i>"]
            US13["[Build Mode] place_symmetries, on_place_message<br/>toggle_advanced_build→compute_and_render<br/><i>structure/shared/build_mode/</i>"]
            US14["[Ship Config] open_config_menu<br/>change_ship_faction_id→attach_ui (.chain)<br/><i>structure/ship/ui/</i>"]
            US15["[Shipyard] on_open_shipyard, on_change_state<br/><i>block/multiblocks/shipyard/</i>"]
            US16["[Reactor UI] maintain_active_text,<br/>update_status_bar, update_generation_stats<br/><i>block/multiblocks/reactor/ui.rs</i>"]
            US17["[Camera System] on_add_camera→swap→<br/>on_change_selected→on_stop_piloting (.chain)<br/><i>structure/systems/camera_system.rs</i>"]
            US18["[Shield Rendering] on_add→on_change_update→<br/>update_shield_times (.chain)<br/><i>structure/shields/mod.rs</i>"]
            US19["[Railgun] on_fire_railgun→fade_railgun_blast (.chain)<br/><i>structure/systems/railgun_system.rs</i>"]
            US20["[Missile Launcher] focus_looking_at→<br/>apply_shooting_sound→render_lockon→listen_errors (.chain)<br/><i>structure/systems/missile_launcher_system.rs</i>"]
            US21["[Laser] apply_shooting_sound<br/><i>structure/systems/laser_cannon_system.rs</i>"]
            US22["on_use_item (.run_if no_open_menus)<br/><i>item/usable/mod.rs</i>"]
            US23["generate_input_events (.run_if no_open_menus)<br/><i>interactions/block_interactions.rs</i>"]
            US24["[Debug] update_coords, update_looking_at, update_fps<br/><i>ui/debug_info_display.rs</i>"]
            US25["[HUD] update_text_to_alignment, create_credits_node<br/>show_error→tick_error (.chain)<br/><i>ui/hud/</i>"]
            US26["[Notifications] on_recv_notification<br/><i>notifications/mod.rs</i>"]
            US27["[Blueprints] on_receive_download, upload_selected<br/><i>item/usable/blueprint.rs</i>"]
            US28["[Quest] on_active_quest, fade_text<br/><i>quest/waypoint.rs + quest/ui/hud.rs</i>"]
            US29["send_render_distance<br/><i>entities/player/render_distance.rs</i>"]
            US30["take_damage_reader<br/><i>structure/events.rs</i>"]
            US31["remove_self_from_structure (.run_if no_open_menus)<br/><i>structure/shared/mod.rs</i>"]
        end

        subgraph U_AUDIO ["Audio (AudioSet)"]
            UA1["CreateSounds"]
            UA1S1["monitor_background_song→adjust_volume (.chain)<br/><i>audio/music/mod.rs</i>"]
            UA1S2["trigger_music_playing→start_playing (.chain)<br/><i>audio/music/dynamic_music.rs</i>"]
            UA1S3["apply_thruster_sound<br/><i>structure/systems/thruster_system.rs</i>"]
            UA2["ProcessSounds"]
            UA2S1["play_block_place_sound→<br/>play_block_break_sound (.chain)<br/><i>structure/audio/break_place_block_sound.rs</i>"]
            UA2S2["play_block_damage_sound<br/><i>structure/audio/take_damage_sound.rs</i>"]
            UA1 --> UA2
        end

        subgraph U_SETTINGS ["Settings"]
            USET1["LoadSettings (.chain)<br/>load_volume (music + master) → load_gamma→<br/>load_mouse_sensitivity→load_fov<br/><i>settings/mod.rs + audio/</i>"]
            USET2["ChangeSettings<br/>on_change_loaded_settings→serialize (.chain)<br/><i>settings/mod.rs</i>"]
            USET1 --> USET2
        end

        subgraph U_MAINMENU ["Main Menu"]
            UM1["CleanupMenu<br/>despawn_all→create_root_node (.chain)<br/><i>ui/main_menu/mod.rs</i>"]
            UM2["InitializeMenu<br/>create_main_menu<br/><i>ui/main_menu/title_screen/mod.rs</i>"]
            UM3["UpdateMenu<br/>spin_camera, fade_in_background<br/><i>ui/main_menu/mod.rs</i>"]
            UM1 --> UM2 --> UM3
        end

        subgraph U_MISC ["Miscellaneous"]
            UMSC1["process_player_camera<br/><i>camera/camera_controller.rs</i>"]
            UMSC2["create_added_star, point_light_from_sun<br/><i>universe/star.rs</i>"]
            UMSC3["create_added_black_hole<br/><i>universe/black_hole.rs</i>"]
            UMSC4["asset_loaded→add_skybox_to_needed (.chain)<br/><i>skybox/mod.rs</i>"]
            UMSC5["add_world_within<br/><i>physics/mod.rs</i>"]
            UMSC6["add_meshes (mesh_delayer)<br/><i>rendering/mesh_delayer.rs</i>"]
            UMSC7["[Panorama] take_panorama→restore_ui (.chain)<br/>taking_panorama<br/><i>rendering/panorama/mod.rs</i>"]
            UMSC8["on_change_controls<br/><i>input/inputs.rs</i>"]
            UMSC9["[Planet LOD] on_add_planet→generate_player_lods→<br/>flag_for_generation→send_chunks_to_gpu→<br/>read_gpu_data→generate_chunks→<br/>on_change_being_generated<br/><i>structure/planet/lods/mod.rs</i>"]
            UMSC10["play_warp_animation<br/><i>structure/systems/warp/warp_drive.rs</i>"]
            UMSC11["color_planet_skybox<br/><i>structure/planet/planet_skybox.rs</i>"]
            UMSC12["[Create Ship/Station] listeners + handlers<br/><i>structure/ship/create_ship.rs</i>"]
            UMSC13["[Connection] connect_to_auto_connect, ensure_connected<br/>remove_networking_resources, wait_for_done_loading<br/>wait_for_connection<br/><i>netty/connect.rs + netty/loading.rs</i>"]
            UMSC14["on_change_looking_at (tooltip)<br/><i>ui/hud/tooltip.rs</i>"]
            UMSC15["[Reactivity] update_reacts<br/>text, slider, node, text_input, bound_value<br/><i>ui/reactivity/</i>"]
            UMSC16["[Friends] add_invite_callback<br/><i>ui/friends/mod.rs</i>"]
            UMSC17["on_add_missile<br/><i>projectiles/missile.rs</i>"]
        end

    end

    %% ==================== PREUPDATE / POSTUPDATE ====================
    subgraph PREUP ["PreUpdate"]
        PR1["[Audio cleanup] stop_audio_sources→monitor_attached→<br/>cleanup_despawning→cleanup_stopped_spacial (.chain)<br/><i>audio/mod.rs</i>"]
        PR2["steam_callbacks<br/><i>netty/steam.rs</i>"]
        PR3["clear_focus_on_hidden<br/><i>ui/components/focus.rs</i>"]
    end

    subgraph POSTUP ["PostUpdate"]
        PO1["[Audio playback] start_playing_audio→<br/>set_volume_to_zero→run_spacial_audio (.chain)<br/><i>audio/mod.rs</i>"]
    end

    subgraph EXTRACT ["ExtractSchedule"]
        EX1["extract_scrollbars<br/><i>ui/components/scollable_container.rs</i>"]
    end

    %% ==================== FIRST ====================
    subgraph FIRST ["First"]
        F1["remove_despawned_entities (.before despawn_needed)<br/><i>netty/gameplay/mod.rs</i>"]
        F2["remove_mappings (.after despawn_needed)<br/><i>ecs/mod.rs</i>"]
    end

    %% ==================== TOP-LEVEL CONNECTIONS ====================
    STARTUP --> FIRST
    STARTUP --> FU
    FU_UNC -.-> FU
    PREUP -.-> UPDATE
    POSTUP -.-> UPDATE
    EXTRACT -.-> UPDATE

    %% ==================== STYLES ====================
    style STARTUP style fill:#1a1a2e,stroke:#e94560,color:#eee
    style SS_START fill:#2a1a3a,stroke:#ab47bc,color:#eee
    style SS_PRELOAD fill:#1a3a2a,stroke:#66bb6a,color:#eee
    style SS_LOAD fill:#2a2a1a,stroke:#9e9d24,color:#eee
    style SS_POSTLOAD fill:#1a2a3a,stroke:#42a5f5,color:#eee
    style SS_PLAY fill:#3a1a1a,stroke:#ef5350,color:#eee
    style FU fill:#16213e,stroke:#0f3460,color:#eee
    style FU_NETTYRECV fill:#533483,stroke:#7b68ee,color:#eee
    style FU_NETTYSEND fill:#533483,stroke:#7b68ee,color:#eee
    style FU_MAIN fill:#1a3a5c,stroke:#3498ff,color:#eee
    style MAIN_IN fill:#2d5a27,stroke:#4caf50,color:#eee
    style MAIN_SIM fill:#5a3d1a,stroke:#ff9800,color:#eee
    style MAIN_EV fill:#5a2d2d,stroke:#f44336,color:#eee
    style MAIN_LT fill:#4a1a5c,stroke:#9c27b0,color:#eee
    style FU_LOCSYNC fill:#1a4a3a,stroke:#00bcd4,color:#eee
    style FU_PREPHYS fill:#3a2d0a,stroke:#ffc107,color:#eee
    style FU_RAPIER fill:#4a0a0a,stroke:#ff5722,color:#eee
    style FU_POSTPHYS fill:#3a2d0a,stroke:#ffc107,color:#eee
    style FU_LOCSYNC2 fill:#1a4a3a,stroke:#00bcd4,color:#eee
    style FU_POSTLOC fill:#4a1a1a,stroke:#e91e63,color:#eee
    style FU_UNC fill:#2d2d2d,stroke:#666,color:#eee
    style FU_BLOCKMSGS fill:#1a3a3a,stroke:#26a69a,color:#eee
    style FU_STRUC fill:#3a1a3a,stroke:#ab47bc,color:#eee
    style FU_PROJ fill:#4a2d0a,stroke:#ff9800,color:#eee
    style FU_LASERS fill:#0a3a4a,stroke:#00bcd4,color:#eee
    style FU_MISC fill:#2a2a1a,stroke:#9e9d24,color:#eee
    style UPDATE fill:#0d1b2a,stroke:#00e676,color:#eee
    style U_NETTY fill:#533483,stroke:#7b68ee,color:#eee
    style U_CURSOR fill:#1a3a5c,stroke:#3498ff,color:#eee
    style U_ASSETSLOADING fill:#1a3a2a,stroke:#66bb6a,color:#eee
    style U_ASSETSREADY fill:#1a3a2a,stroke:#8bc34a,color:#eee
    style U_RENDER fill:#3a2d0a,stroke:#ffc107,color:#eee
    style U_UI fill:#0a3a4a,stroke:#00bcd4,color:#eee
    style UU_PRE fill:#2d5a27,stroke:#4caf50,color:#eee
    style UU_DO fill:#1a3a5c,stroke:#3498ff,color:#eee
    style UU_FIN fill:#5a2d2d,stroke:#f44336,color:#eee
    style U_UI_UNSET fill:#2d2d2d,stroke:#78909c,color:#eee
    style U_AUDIO fill:#4a1a5c,stroke:#9c27b0,color:#eee
    style U_SETTINGS fill:#1a2a4a,stroke:#42a5f5,color:#eee
    style U_MAINMENU fill:#5a1a3a,stroke:#e91e63,color:#eee
    style U_MISC fill:#2a2a1a,stroke:#9e9d24,color:#eee
    style PREUP fill:#0a2a2a,stroke:#26c6da,color:#eee
    style POSTUP fill:#2a0a2a,stroke:#ba68c8,color:#eee
    style EXTRACT fill:#1a1a1a,stroke:#78909c,color:#eee
    style FIRST fill:#2a1a0a,stroke:#ff8f00,color:#eee
```

## Legend

| Color | Schedule / Set |
|-------|---------------|
| Purple | Networking (Receive / Send) |
| Blue | FixedUpdateSet::Main |
| Green (dark) | InputProcessing |
| Orange | Simulation |
| Red | EventProcessing |
| Purple (dark) | Late |
| Teal | LocationSyncing |
| Yellow | PrePhysics / PostPhysics |
| Red (dark) | Rapier Physics |
| Pink | PostLocationSyncingPostPhysics |
| Gray | Unconstrained / Misc |
| Green (bright) | Asset Loading / Ready |
| Cyan | UI Systems (UISystemSet) |
| Amber | Structure Rendering |
| Magenta | Audio / Extract |

## Pipeline Summary

```
Startup → Loading → FixedUpdate (60 Hz):
  NettyReceive → Main (InputProcessing → Simulation → EventProcessing → Late)
  → LocationSyncing → PrePhysics → [Rapier] → PostPhysics
  → LocationSyncingPostPhysics → PostLocationSyncingPostPhysics → NettySend
                               ↕
  Unconstrained systems alongside: block events, structure systems,
  projectiles, mining lasers, loading

UPDATE (variable rate):
  Networking → Cursor/Window → [AssetsSet chain] → [StructureRenderingSet chain]
  → [UISystemSet: PreDoUi → DoUi → FinishUi] → [AudioSet: Create → Process]
  → [SettingsSet: Load → Change] → [MainMenuSet: Cleanup → Init → Update]
  → Misc: camera, skybox, planet LOD, connection, crosshair, tooltip, ...
```

## Key Differences from Server

| Aspect | Server | Client |
|--------|--------|--------|
| MainSet::Simulation | 28 systems (AI, physics, logic) | 2 systems (planet alignment, rotation) |
| MainSet::InputProcessing | empty | 4 systems (camera, movement, blueprint, ship input) |
| MainSet::Late | empty | 2 systems (quest UI, warp audio) |
| MainSet::EventProcessing | 18 systems (health, faction, chat, etc.) | 5 systems (recipe sync, player add/teleport/respawn) |
| UPDATE schedule | ~15 systems (chat, shop, respawn) | **~130+ systems** (UI, audio, rendering, settings, menus, etc.) |
| Asset loading | minimal | extensive (textures, meshes, audio, materials, LODs) |
| SAVING_SCHEDULE | present | absent |
| LOADING_SCHEDULE | present | absent |
| PreUpdate / PostUpdate | minimal | audio cleanup/playback chains |
