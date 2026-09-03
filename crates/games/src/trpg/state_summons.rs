impl TacticalState {
    pub fn spawn_owned_unit(
        &mut self,
        summoner: AgentId,
        job_name: &str,
        position: GridCell,
        maximum_active: u8,
    ) -> Result<AgentId, String> {
        let owner = self
            .agents
            .get(&summoner)
            .filter(|owner| owner.health > 0)
            .cloned()
            .ok_or_else(|| format!("Unknown or dead summoner {}", summoner.0))?;
        if !self.grid.contains(position) {
            return Err("Summon cell is not supported by the tactical grid".to_string());
        }
        if self
            .agents
            .values()
            .any(|unit| unit.health > 0 && unit.position == position)
        {
            return Err("Summon cell is occupied".to_string());
        }
        let active = self
            .summons
            .iter()
            .filter(|(id, info)| {
                info.summoner == summoner && self.agents.get(id).is_some_and(|unit| unit.health > 0)
            })
            .count();
        if active >= usize::from(maximum_active) {
            return Err(format!(
                "Summoner {} already has the maximum {} active summons",
                summoner.0, maximum_active
            ));
        }
        let (job_id, job) = self
            .job_registry
            .iter()
            .find(|(_, job)| job.name == job_name)
            .map(|(id, job)| (*id, job.clone()))
            .ok_or_else(|| format!("Unknown summoned job: {job_name}"))?;
        let id = AgentId(self.next_agent_id);
        if self.agents.contains_key(&id) {
            return Err(format!("Summon ID {} is already in use", id.0));
        }
        let spawn_sequence = self.next_spawn_sequence;
        self.next_agent_id = self
            .next_agent_id
            .checked_add(1)
            .ok_or_else(|| "Summon unit ID allocator overflowed".to_string())?;
        self.next_spawn_sequence = self
            .next_spawn_sequence
            .checked_add(1)
            .ok_or_else(|| "Summon sequence allocator overflowed".to_string())?;
        let unit = UnitState {
            team_id: owner.team_id,
            health: job.base_stats.constitution * 10,
            mana: job.base_stats.intelligence * 5,
            action_points: 4,
            ct: 0,
            position,
            facing: Facing::default(),
            gender: Gender::Nonbinary,
            class_id: ActorClassId(1),
            primary_job: job_id,
            secondary_jobs: Vec::new(),
            job_history: vec![JobHistoryEntry {
                job_id,
                consecutive_levels: 1,
            }],
            purchased_abilities: Vec::new(),
            movement_ability: job.movement,
            passive_abilities: job.passives.clone(),
            reaction_abilities: job.reactions.clone(),
            stats: job.base_stats.clone(),
            equipment: EquipmentSlots {
                slots: HashMap::new(),
            },
            status_effects: Vec::new(),
            turn_tags: TagBag::default(),
            modifier_deck: AbilityModifierDeck::default(),
            timed_modifiers: Vec::new(),
            derived_stats: DerivedStats {
                health_max: job.base_stats.constitution * 10,
                mana_max: job.base_stats.intelligence * 5,
                action_points_max: 4,
            },
        };
        self.agents.insert(id, unit);
        self.controllers.insert(
            id,
            self.summon_controllers
                .get(&summoner)
                .copied()
                .unwrap_or(UnitController::Npc),
        );
        self.summons.insert(
            id,
            SummonInfo {
                summoner,
                job: job_id,
                spawn_sequence,
            },
        );
        Ok(id)
    }

    pub fn cleanup_orphaned_summons(&mut self) -> Vec<AgentId> {
        let mut orphaned = self
            .summons
            .iter()
            .filter(|(_, info)| {
                self.agents
                    .get(&info.summoner)
                    .is_none_or(|owner| owner.health <= 0)
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        orphaned.sort();
        for id in &orphaned {
            if let Some(unit) = self.agents.get_mut(id) {
                unit.health = 0;
            }
        }
        orphaned
    }
}
