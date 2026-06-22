# Cosmos Server — System Ordering

```mermaid
graph TD
    %% ==================== STARTUP ====================
    subgraph STARTUP ["Startup / State Transitions"]
        S1["load_world_path<br/><i>persistence/mod.rs</i>"]
        S2["load_operators<br/><i>commands/operator.rs</i>"]
        S3["load_player_whitelist/blacklist<br/><i>netty/player_filtering.rs</i>"]
        S4["add_shipyard_state_hooks<br/><i>blocks/.../shipyard/impls.rs</i>"]
        S_ENTER_PRE["OnEnter: PreLoading<br/>register_resource (BlockDrops)"]
        S_ENTER_LOAD["OnEnter: Loading<br/>register_logic_groups<br/>register_quest<br/>register_commands"]
        S_ENTER_POST["OnEnter: PostLoading<br/>load_factions, load_recipes, register_power_blocks<br/>register_fluid_blocks, register_energy_blocks, ..."]
        S_ENTER_PLAY["OnEnter: Playing<br/>display_basic_info<br/>make_local_player_op"]
    end

    %% ==================== FIXEDUPDATE PIPELINE ====================
    subgraph FU ["FIXEDUPDATE — 60 Hz pipeline (.chain)"]

        subgraph FU_NETTYRECV ["FixedUpdateSet::NettyReceive"]
            NR_RECV["NetworkingSystemsSet::ReceiveMessages"]
            NR_RECV --> NR_PROC["NetworkingSystemsSet::ProcessReceivedMessages"]
            NR_RECV_S1["server_listen_messages<br/><i>netty/server_listener.rs</i>"]
            NR_RECV_S2["handle_pre_connect_messages →<br/>on_change_preconnect_player<br/><i>netty/server_events.rs</i>"]
            NR_RECV_S3["listen_for_done_syncing<br/><i>netty/sync/registry.rs</i>"]
            NR_RECV --> NR_RECV_S1
            NR_RECV --> NR_RECV_S2
            NR_RECV --> NR_RECV_S3
        end

        subgraph FU_MAIN ["FixedUpdateSet::Main"]
            subgraph MAIN_INPUT ["MainSet::InputProcessing"]
                MI_NOTE["(empty on server)"]
            end

            subgraph MAIN_SIM ["MainSet::Simulation"]
                MS1["[ProcessCommands] command_receiver →<br/>warn_on_no_command_hit<br/><i>commands/cosmos_command_handler.rs</i>"]
                MS2["clear_movement_when_no_pilot<br/><i>cosmos_core/structure/ship/ship_movement.rs</i>"]
                MS3["on_change_system_slot<br/><i>structure/systems/system_ordering.rs</i>"]
                MS4["pickup_near_item →<br/>advance_time_since_spawn<br/><i>items/mod.rs</i>"]
                MS5["process_activate_reactor<br/><i>blocks/multiblock/reactor/impls.rs</i>"]
                MS6["handle_combat_ai<br/><i>ai/combat.rs</i>"]
                MS7["[PirateAI] 6 systems<br/><i>ai/pirate/ship_ai.rs</i>"]
                MS8["[MerchantAI] 11 systems<br/><i>ai/quest_npc/mod.rs</i>"]
                MS9["on_add_quest → on_kill_pirates<br/><i>quest/quests/fight_pirate.rs</i>"]
                MS10["[COMS] 7 systems<br/><i>coms/systems.rs</i>"]
            end

            subgraph MAIN_EVENT ["MainSet::EventProcessing"]
                ME1["on_change_health, regenerate_health<br/><i>entities/health.rs</i>"]
                ME2["on_die<br/><i>cosmos_core/entities/health.rs</i>"]
                ME3["[Faction] 6 systems<br/><i>faction/events.rs</i>"]
                ME6["on_trash_item_creative →<br/>on_grab_creative_item<br/><i>creative/mod.rs</i>"]
                ME7["on_primary_player_disconnect<br/><i>local/mod.rs</i>"]
                ME8["[PilotEvents] 6 systems<br/><i>cosmos_core/.../pilot_change_event_listener.rs</i>"]
            end

            subgraph MAIN_LATE ["MainSet::Late"]
                ML_NOTE["(empty on server)"]
            end

            MAIN_INPUT --> MAIN_SIM --> MAIN_EVENT --> MAIN_LATE
        end

        subgraph FU_LOCSYNC ["FixedUpdateSet::LocationSyncing"]
            LS1["look_and_move_towards_target<br/><i>cosmos_core/projectiles/missile.rs</i>"]
            LS2["apply_missile_thrust<br/><i>cosmos_core/projectiles/missile.rs</i>"]
        end

        subgraph FU_PREPHYS ["FixedUpdateSet::PrePhysics<br/><i>.before PhysicsSet::SyncBackend</i>"]
            PP1["[Shipyard] 5 systems (.chain)<br/>place→change_state→interact→<br/>dont_move→create_state<br/><i>blocks/.../shipyard/impls.rs</i>"]
        end

        subgraph RAPIER ["Rapier Physics (3 phases)"]
            RP1["PhysicsSet::SyncBackend"]
            RP2["PhysicsSet::StepSimulation"]
            RP3["PhysicsSet::Writeback"]
            RP1 --> RP2 --> RP3
        end

        subgraph FU_POSTPHYS ["FixedUpdateSet::PostPhysics<br/><i>.after PhysicsSet::Writeback</i>"]
            POP1["restore_position<br/><i>cosmos_core/physics/disable_rigid_body.rs</i>"]
        end

        subgraph FU_LOCSYNC2 ["FixedUpdateSet::LocationSyncingPostPhysics"]
            LS2_1["send_laser_hit_events →<br/>despawn_lasers<br/><i>cosmos_core/projectiles/laser.rs</i>"]
            LS2_2["log_position<br/><i>cosmos_core/physics/disable_rigid_body.rs</i>"]
        end

        subgraph FU_POSTLOC ["FixedUpdateSet::PostLocationSyncingPostPhysics"]
            PL1["respond_to_collisions →<br/>despawn_missiles<br/><i>cosmos_core/projectiles/missile.rs</i>"]
            PL2["respond_to_explosion<br/><i>cosmos_core/projectiles/explosion.rs</i>"]
            PL3["respond_laser_hit_event<br/><i>cosmos_core/projectiles/laser.rs</i>"]
            PL4["disable_colliders<br/><i>physics/collider_disabling.rs</i>"]
            PL5["disable_rigid_bodies<br/><i>cosmos_core/physics/disable_rigid_body.rs</i>"]
        end

        subgraph FU_NETTYSEND ["FixedUpdateSet::NettySend"]
            NS_SF["[SyncFlags] 5 systems (.chain)<br/>add_item_data_sync_flag →<br/>add_structure_systems_sync_flag →<br/>on_needs_sync_data →<br/>update_sync_players →<br/>generate_request_entity_events<br/><i>netty/sync/flags.rs</i>"]
            NS_SC["NetworkingSystemsSet::SyncComponents"]
            NS_SC_S1["server_sync_bodies<br/><i>netty/sync/sync_bodies.rs</i>"]
            NS_SC_S2["on_request_parent→change→remove<br/><i>netty/sync/components/parent.rs</i>"]
            NS_SC_S3["handle_block_changed_event<br/><i>blocks/block_events.rs</i>"]
            NS_SC_S4["monitor_pilot_changes<br/>monitor_set_movement_events<br/><i>structure/ship/events.rs</i>"]
            NS_SC_S5["sync_recipes_on_join→change (×2)<br/><i>crafting/recipes/</i>"]
            NS_SC_S6["send_shield_hits<br/><i>structure/systems/shield_system/</i>"]
            NS_SC_S7["send_messages (commands)<br/><i>commands/cosmos_command_handler.rs</i>"]
            NS_SC_S8["on_request_under_grav<br/><i>blocks/interactable/gravity_well.rs</i>"]
            NS_SF --> NS_SC
            NS_SC --> NS_SC_S1
            NS_SC --> NS_SC_S2
            NS_SC --> NS_SC_S3
            NS_SC --> NS_SC_S4
            NS_SC --> NS_SC_S5
            NS_SC --> NS_SC_S6
            NS_SC --> NS_SC_S7
            NS_SC --> NS_SC_S8
        end

        FU_NETTYRECV --> FU_MAIN --> FU_LOCSYNC --> FU_PREPHYS --> RAPIER --> FU_POSTPHYS --> FU_LOCSYNC2 --> FU_POSTLOC --> FU_NETTYSEND
    end

    %% ==================== UNCONSTRAINED FIXEDUPDATE ====================
    subgraph FU_UNCONSTRAINED ["Unconstrained FixedUpdate — run alongside pipeline"]

        subgraph LOGIC ["NetworkingSystemsSet::Between"]
            LOGIC_CHAIN["[LogicSystemSet] (.chain)<br/>PreLogicTick→EditLogicGraph→<br/>QueueConsumers→QueueProducers→<br/>SendQueues→Consume→Produce<br/><i>logic/mod.rs</i>"]
        end

        subgraph BLOCKMSGS ["BlockMessagesSet (.chain)"]
            BM_DIRECTION["SendForThisFrame ► PreProcess ► SendBlockUpdate ►<br/>PrePlacement ► ChangeBlocks ► Process ►<br/>PostProcess ► SendForNextFrame"]
            BM_R1["on_place_tank<br/><i>fluid/mod.rs</i>"]
            BM_R2["handle_block_event (reactor, door, dye,<br/>ship_core, storage)<br/><i>blocks/interactable/</i>"]
            BM_R3["monitor fabricator interactions (×2)<br/><i>crafting/blocks/</i>"]
            BM_R4["add_hitters→process_hit_events→<br/>add_faction_enemies<br/><i>ai/hit_tracking/</i>"]
            BM_R5["on_modify_reactor→generate_power<br/><i>blocks/.../reactor/impls.rs</i>"]
            BM_R6["recalculate_shields→...→power_shields<br/><i>structure/systems/shield_system/</i>"]
            BM_R7["handle_block_break→place (.chain)<br/><i>blocks/block_events.rs</i>"]
            BM_R8["on_interact_reactor<br/><i>blocks/.../reactor/mod.rs</i>"]
            BM_R9["on_add_tank→listen_changed→balance<br/><i>fluid/tank.rs</i>"]
            BM_R10["toggle_doors, monitor_grass_updated<br/>on_melting_down<br/><i>blocks/interactable/door.rs</i>"]
        end

        subgraph STR_LOAD ["StructureLoadingSet (.chain, .before NettySend)"]
            SL1["manage_shipyards→on_set_blueprint<br/><i>blocks/.../shipyard/impls.rs</i>"]
            SL2["create_ships<br/><i>structure/ship/loading.rs</i>"]
            SL3["create_ship_event_reader<br/><i>structure/ship/events.rs</i>"]
        end

        subgraph PLAYER_STR ["Player Strength"]
            PS1["add_total_time_played, add_player_strength<br/><i>entities/player/strength.rs</i>"]
            PS2["advance_total_time (on_timer 1s)<br/><i>entities/player/strength.rs</i>"]
        end

        subgraph QUESTS ["Quest Systems"]
            Q1["add_ongoing_quests<br/><i>quest/mod.rs</i>"]
            Q2["clear_invalid_active→on_set_ongoing (.chain)<br/><i>quest/mod.rs</i>"]
            Q3["on_complete_quest<br/><i>quest/mod.rs</i>"]
        end

        subgraph PLAYER_LOAD ["Player Loading (.in_set Between)"]
            PLL1["unload_far<br/><i>persistence/player_loading.rs</i>"]
            PLL2["load_near (on_timer 1s)<br/><i>persistence/player_loading.rs</i>"]
            PLL3["recompute_need_loaded_children<br/><i>persistence/player_loading.rs</i>"]
            PLL4["monitor_loading_task<br/><i>persistence/player_loading.rs</i>"]
        end

        subgraph MISC ["Miscellaneous"]
            MC1["on_update_operators→on_update_players<br/><i>commands/operator.rs</i>"]
            MC2["grav_well→remove_gravity→sync (.chain)<br/><i>blocks/interactable/gravity_well.rs</i>"]
            MC3["tick_down_hitters, on_melt_down<br/><i>ai/hit_tracking/</i>"]
            MC4["save_whitelist / save_blacklist<br/><i>netty/player_filtering.rs</i>"]
            MC5["cleanup_backups (on_timer 20min)<br/><i>persistence/backup.rs</i>"]
            MC6["on_add_faction_territory<br/><i>universe/mod.rs</i>"]
            MC7["on_stop_server<br/><i>server/stop.rs</i>"]
        end
    end

    %% ==================== SAVING SCHEDULE ====================
    subgraph SAVING ["SAVING_SCHEDULE (= First)"]

        subgraph BLUEPRINTING ["BlueprintingSystemSet (.chain, .before BeginSaving)"]
            BP1["BeginBlueprinting<br/>check_needs_blueprinted"]
            BP2["DoBlueprinting<br/>on_blueprint_ship"]
            BP3["DoneBlueprinting<br/>done_blueprinting"]
            BP1 --> BP2 --> BP3
        end

        subgraph BACKUP ["Backup"]
            BA1["trigger_autosave→backup_before_saving<br/><i>persistence/autosave.rs</i>"]
            BA2["backup_world<br/><i>persistence/backup.rs</i>"]
            BA3["save_everything<br/><i>persistence/autosave.rs</i>"]
            BA1 --> BA2 --> BA3
        end

        subgraph SAVING_SET ["SavingSystemSet (.chain, .before despawn_needed)"]
            SV1["MarkSavable<br/>mark_savable_entities"]
            SV2["BeginSaving<br/>save_data_entities→check_needs_saved"]
            SV3["CreateEntityIds<br/>create_entity_ids"]
            SV4["DoSaving<br/>save_component::&lt;T&gt; (per PersistentComponent)<br/>save_component_block_data::&lt;T&gt;<br/>save_component_itemstack_data::&lt;T&gt;<br/>default_save<br/>on_save_ai_controlled, on_save_pirate<br/>on_save_laser, on_save_shield<br/>on_save_inventory→serialize_inventory<br/>on_save_ship"]
            SV5["DoneSaving<br/>ensure_data_entities_have_correct_parents<br/>done_saving<br/>save_player_link"]
            SV1 --> SV2 --> SV3 --> SV4 --> SV5
        end

        subgraph SAVING_CLEANUP ["Cleanup"]
            SC1["notify_despawned_entities<br/><i>netty/sync/sync_bodies.rs</i>"]
            SC2["despawn_needed<br/><i>cosmos_core/ecs/mod.rs</i>"]
            SC1 --> SC2
        end

        subgraph SAVING_STOP ["Stop Server"]
            SSTOP["shut_server_down<br/><i>server/stop.rs</i>"]
        end

        BACKUP --> BLUEPRINTING --> SAVING_SET --> SAVING_STOP --> SAVING_CLEANUP
    end

    %% ==================== LOADING SCHEDULE ====================
    subgraph LOADING ["LOADING_SCHEDULE (= FixedUpdate, runs before normal FixedUpdate)"]
        subgraph LOAD_START ["Player Load (.chain)"]
            LPL1["load_player→create_new_player<br/><i>entities/player/persistence.rs</i>"]
        end

        subgraph LOAD_BEGIN ["LoadingSystemSet::BeginLoading"]
            LB1["check_needs_loaded<br/><i>persistence/loading.rs</i>"]
            LB2["check_blueprint_needs_loaded→<br/>load_blueprint_rotation<br/><i>persistence/loading.rs</i>"]
        end

        subgraph LOAD_BASIC ["LoadingSystemSet::LoadBasicComponents"]
            LBC1["load_component::&lt;T&gt;<br/><i>persistence/make_persistent.rs</i>"]
        end

        subgraph LOAD_DO ["LoadingSystemSet::DoLoading"]
            LD1["load_component_from_block_data::&lt;T&gt;<br/><i>persistence/make_persistent.rs</i>"]
            LD2["on_load_ai_controlled, on_load_pirate<br/><i>ai/</i>"]
            LD3["on_load_ship, on_load_shield<br/><i>structure/ship/persistence.rs</i>"]
            LD4["deserialize_inventory→<br/>deserialize_inventory_block_data<br/><i>inventory/mod.rs</i>"]
            LD5["default_load<br/><i>persistence/loading.rs</i>"]
        end

        subgraph LOAD_DONE ["LoadingSystemSet::DoneLoading"]
            LDN1["done_loading<br/><i>persistence/loading.rs</i>"]
            LDN2["done_loading_blueprint<br/><i>persistence/loading.rs</i>"]
            LDN3["finish_loading_player→add_save_link→<br/>name_player_save_links<br/><i>entities/player/persistence.rs</i>"]
        end

        LOAD_START --> LOAD_BEGIN --> LOAD_BASIC --> LOAD_DO --> LOAD_DONE
    end

    %% ==================== UPDATE ====================
    subgraph UPDATE ["UPDATE Schedule"]
        U1["monitor_inputs<br/><i>commands/cosmos_command_handler.rs</i>"]
        U2["send_all_chunks<br/><i>netty/server_listener.rs</i>"]
        U3["close_server_after_ticks<br/><i>converters/mod.rs</i>"]
        U4["event_listener (pilot change)<br/><i>structure/ship/change_pilot_event_listener.rs</i>"]
        U5["on_request_ship<br/><i>structure/ship/sync.rs</i>"]
        U6["[Shop] on_interact→listen_messages→<br/>listen_buy→listen_sell (.chain)<br/><i>shop/ev_reader.rs</i>"]
        U7["[Melting] monitor_block_events<br/><i>structure/shared/melt_down.rs</i>"]
        U8["on_respawn (.before DoPhysics)<br/><i>entities/player/respawn.rs</i>"]
        U9["on_die_drop_items (.after HealthSet)<br/><i>entities/player/respawn.rs</i>"]
        U10["add_spawner→spawn_pirates (on_timer 1s)<br/><i>ai/pirate/station.rs</i>"]
        U11["receive_messages (chat)<br/><i>chat/text_chat.rs</i>"]
        U12["save_factions_on_change<br/><i>faction/mod.rs</i>"]
        U13["dont_save_far (on_timer 5s)<br/><i>structure/asteroid/dynamic.rs</i>"]
    end

    %% ==================== PERSISTENT COMPONENTS ====================
    subgraph PERSIST ["make_persistent::&lt;T&gt; (generated per-type)"]
        PTEXT["Each call to make_persistent::&lt;T&gt; adds 5–6 systems<br/>across SAVING_SCHEDULE and LOADING_SCHEDULE<br/><br/>Types: Credits, FactionId, TimeSinceSpawn, PhysicalItem, Health,<br/>MaxHealth, Dead, PlayerLooking, Creative, PlayerStrength,<br/>TotalTimePlayed, PlayerSaveLink, OngoingQuests, ActiveQuest,<br/>TutorialState, FightPirateQuestNPC, CombatAi, MeltingDown,<br/>GravityWell, Reactors, Reactor, ReactorFuelConsumption,<br/>ReactorActive, BlockFluidData, FluidItemData, BlockLogicData,<br/>SystemEnabled, ShieldSystem, ShieldDowntime, PirateStation,<br/>FactionClaimedTerritory, DataFor, +~15 more"]
    end

    %% ==================== TOP-LEVEL CONNECTIONS ====================
    STARTUP --> LOADING
    LOADING --> FU
    FU_UNCONSTRAINED -.-> FU
    SAVING -.-> FU
    UPDATE -.-> FU

    %% ==================== STYLES ====================
    style STARTUP fill:#1a1a2e,stroke:#e94560,color:#eee
    style FU fill:#16213e,stroke:#0f3460,color:#eee
    style FU_NETTYRECV fill:#533483,stroke:#7b68ee,color:#eee
    style FU_NETTYSEND fill:#533483,stroke:#7b68ee,color:#eee
    style FU_MAIN fill:#1a3a5c,stroke:#3498ff,color:#eee
    style MAIN_INPUT fill:#2d5a27,stroke:#4caf50,color:#eee
    style MAIN_SIM fill:#5a3d1a,stroke:#ff9800,color:#eee
    style MAIN_EVENT fill:#5a2d2d,stroke:#f44336,color:#eee
    style MAIN_LATE fill:#4a1a5c,stroke:#9c27b0,color:#eee
    style FU_LOCSYNC fill:#1a4a3a,stroke:#00bcd4,color:#eee
    style FU_PREPHYS fill:#3a2d0a,stroke:#ffc107,color:#eee
    style RAPIER fill:#4a0a0a,stroke:#ff5722,color:#eee
    style FU_POSTPHYS fill:#3a2d0a,stroke:#ffc107,color:#eee
    style FU_LOCSYNC2 fill:#1a4a3a,stroke:#00bcd4,color:#eee
    style FU_POSTLOC fill:#4a1a1a,stroke:#e91e63,color:#eee
    style FU_UNCONSTRAINED fill:#2d2d2d,stroke:#666,color:#eee
    style LOGIC fill:#3a3a1a,stroke:#cddc39,color:#eee
    style BLOCKMSGS fill:#1a3a3a,stroke:#26a69a,color:#eee
    style STR_LOAD fill:#3a1a3a,stroke:#ab47bc,color:#eee
    style PLAYER_STR fill:#1a2a4a,stroke:#42a5f5,color:#eee
    style QUESTS fill:#2a1a3a,stroke:#7e57c2,color:#eee
    style PLAYER_LOAD fill:#1a3a2a,stroke:#66bb6a,color:#eee
    style MISC fill:#2a2a1a,stroke:#9e9d24,color:#eee
    style SAVING fill:#0d1b2a,stroke:#1b98f5,color:#eee
    style BLUEPRINTING fill:#1a2d3a,stroke:#29b6f6,color:#eee
    style BACKUP fill:#2a1d2a,stroke:#ba68c8,color:#eee
    style SAVING_SET fill:#0a2a2a,stroke:#26c6da,color:#eee
    style SAVING_CLEANUP fill:#2a1a1a,stroke:#ef5350,color:#eee
    style SAVING_STOP fill:#2a0a0a,stroke:#ff1744,color:#eee
    style LOADING fill:#0d1b2a,stroke:#00e676,color:#eee
    style LOAD_START fill:#2a3a1a,stroke:#8bc34a,color:#eee
    style LOAD_BEGIN fill:#2a3a1a,stroke:#8bc34a,color:#eee
    style LOAD_BASIC fill:#2a3a1a,stroke:#8bc34a,color:#eee
    style LOAD_DO fill:#2a3a1a,stroke:#8bc34a,color:#eee
    style LOAD_DONE fill:#2a3a1a,stroke:#8bc34a,color:#eee
    style UPDATE fill:#1a1a2e,stroke:#ffab40,color:#eee
    style PERSIST fill:#1a1a1a,stroke:#78909c,color:#eee
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
| Gray | Unconstrained FixedUpdate |
| Dark blue | Saving / Loading schedules |
| Amber | Update schedule |

## Pipeline Summary

```
Startup → Loading → FixedUpdate (60 Hz):
  NettyReceive → Main (InputProcessing → Simulation → EventProcessing → Late)
  → LocationSyncing → PrePhysics → [Rapier] → PostPhysics
  → LocationSyncingPostPhysics → PostLocationSyncingPostPhysics → NettySend
                              ↕
  Unconstrained systems run alongside (BlockMessages, StructureLoading,
  Logic, PlayerLoading, Quests, Misc)
                              ↕
  SAVING_SCHEDULE (First) runs between ticks:
  Blueprinting → Backup → Saving (Mark→Begin→CreateIds→Do→Done) → Shutdown → Cleanup
```
