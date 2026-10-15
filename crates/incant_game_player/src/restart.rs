//! Host chronology is independent of the replaceable game's simulation clock.
//! Only native CPU programs opt in; old module/GPU hosts keep typed refusal.
mod input;
use crate::session::{Player, Result, Settings};
use incant_game_runtime::GameSeed;
use incant_script::{SessionProgram, SessionRequest};
pub(crate) use input::InputFence;
use serde::Serialize;

#[derive(Serialize)]
pub struct Boundary {
    host_step: u64,
    completed_generation: u64,
    simulation_tick: u64,
    output_frames: String,
}
pub struct Restart {
    seed: Option<GameSeed>,
    #[cfg(feature = "render-input")]
    settings: Option<incant_render_game::NativeRenderSettings>,
    pub input: InputFence,
    pub generation: u64,
    pub boundaries: Vec<Boundary>,
}
impl Restart {
    pub fn prepare(
        scene: &incant_runtime::CookedScene,
        program: &SessionProgram,
        actions: &incant_input::InputActions,
        clips: &[incant_audio_runtime::ClipBinding],
        settings: &Option<Settings>,
    ) -> Result<Self> {
        let supported = match settings {
            None => true,
            #[cfg(feature = "render-input")]
            Some(Settings::Cpu(_)) => true,
            #[cfg(feature = "render-input")]
            Some(Settings::Recording(_, _)) => false,
            #[cfg(feature = "render-gpu")]
            Some(Settings::Gpu(_) | Settings::RecordedGpu(_, _)) => false,
            #[cfg(not(feature = "render-input"))]
            Some(settings) => match *settings {},
        };
        #[allow(unreachable_patterns)] // all-native builds have just one program kind
        let seed = match program {
            SessionProgram::Bytecode(script) if supported => Some(GameSeed::new(
                scene.clone(),
                script.clone(),
                actions.clone(),
                clips.to_vec(),
            )?),
            _ => None,
        };
        Ok(Self {
            seed,
            #[cfg(feature = "render-input")]
            settings: match settings {
                Some(Settings::Cpu(s)) => Some(copy_settings(s)),
                _ => None,
            },
            input: InputFence::default(),
            generation: 1,
            boundaries: Vec::new(),
        })
    }
    pub fn enable(&self, game: &mut Player) -> Result<()> {
        if self.seed.is_some() {
            game.enable_restart_requests()?;
        }
        Ok(())
    }
    pub fn viewport(&mut self, viewport: Option<[f32; 2]>) {
        #[cfg(feature = "render-input")]
        if let (Some(settings), Some(viewport)) = (&mut self.settings, viewport)
            && let Some(ui) = &mut settings.ui
        {
            ui.viewport = viewport;
        }
        #[cfg(not(feature = "render-input"))]
        let _ = viewport;
    }
    /// Call strictly after the complete accepted output and its observations.
    /// Reconstruction uses no save data and destroys the old composite first.
    pub fn accept(&mut self, game: &mut Player, step: u64, frames: u128) -> Result<bool> {
        if game.take_session_request()? != Some(SessionRequest::Restart) {
            return Ok(false);
        }
        let seed = self
            .seed
            .as_ref()
            .ok_or("restart request without a startup seed")?;
        let next = self
            .generation
            .checked_add(1)
            .ok_or("session generation exhausted")?;
        if self.boundaries.len() == 3600 {
            return Err("restart history limit".into());
        }
        let boundary = Boundary {
            host_step: step,
            completed_generation: self.generation,
            simulation_tick: game.world()?.tick(),
            output_frames: frames.to_string(),
        };
        match game {
            Player::Plain(game) => game.restart_from(seed)?,
            #[cfg(feature = "render-input")]
            Player::Spatial(game) => game.restart_from(
                seed,
                copy_settings(
                    self.settings
                        .as_ref()
                        .ok_or("missing startup host settings")?,
                ),
            )?,
            #[cfg(feature = "render-input")]
            Player::Recording(_) => return Err("material recording restart is unsupported".into()),
            #[cfg(feature = "render-gpu")]
            Player::Gpu(_) | Player::RecordedGpu(_) => {
                return Err("offscreen GPU restart is unsupported".into());
            }
        }
        self.input.replace_session();
        self.generation = next;
        self.boundaries.push(boundary);
        Ok(true)
    }
}
#[cfg(feature = "render-input")]
fn copy_settings(
    s: &incant_render_game::NativeRenderSettings,
) -> incant_render_game::NativeRenderSettings {
    incant_render_game::NativeRenderSettings {
        model_cache: s.model_cache.clone(),
        texture_cache: s.texture_cache.clone(),
        camera: s.camera,
        text: s.text.clone(),
        ui: s.ui,
        consumer: (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_runtime::{CookedEntity, CookedScene, StableId};
    use incant_script::CompiledScript;
    #[test]
    fn accepted_requests_reset_simulation_while_fractional_output_and_host_steps_continue() {
        let scene = CookedScene::new(
            StableId([9; 16]),
            144,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap();
        let program = SessionProgram::Bytecode(
            CompiledScript::compile(
                r#"
let calls=0;exports.default={update(t){calls++;if(calls!==t.clock.tick)throw Error('stale closure');
 t.chunks[0].transforms[0]=calls;if(calls===3)t.commands.requestRestart();}}"#,
            )
            .unwrap(),
        );
        let mut restart =
            Restart::prepare(&scene, &program, &Default::default(), &[], &None).unwrap();
        let mut game =
            Player::new(scene, program, Default::default(), vec![], 44117, None, 8).unwrap();
        restart.enable(&mut game).unwrap();
        let mut ticks = vec![];
        let mut frames = 0u128;
        for step in 1..=8 {
            let count = game.next_output_frames().unwrap();
            let mut pcm = vec![1.; count * 2];
            game.tick(&[], &mut pcm).unwrap();
            assert!(pcm.iter().all(|v| *v == 0.));
            ticks.push(game.world().unwrap().tick());
            frames += count as u128;
            restart.accept(&mut game, step, frames).unwrap();
        }
        assert_eq!(ticks, [1, 2, 3, 1, 2, 3, 1, 2]);
        assert_eq!(restart.generation, 3);
        assert_eq!(restart.boundaries.len(), 2);
        assert_eq!(restart.boundaries[0].host_step, 3);
        assert_eq!(restart.boundaries[1].host_step, 6);
        assert_eq!(frames, 2 * (3 * 44117u128 / 144) + 2 * 44117u128 / 144);
        assert_eq!(game.world().unwrap().tick(), 2);
    }
}
