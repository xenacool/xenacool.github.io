const { test, expect } = require('@playwright/test');
const {
  loadWithFixture,
  sendAcceptedAction,
  waitForPlayerBoundary,
} = require('../helpers');
const sendAccepted = sendAcceptedAction;

async function waitForNecromancer(page, predicate = () => true) {
  await page.waitForFunction((predicateSource) => {
    const state = window.__pystralLastTransientState;
    const necromancer = state?.unit_states?.find(({ unit_id }) => unit_id === 5)?.state;
    const status = window.__pystralWorkerStatus || '';
    return state?.active_unit_id === 5
      && state.available_actions
      && state.input_enabled
      && !state.action_pending
      && !state.wait_pending
      && !state.game_completed
      && status.includes('AwaitingPlayerDecision')
      && status.includes('simulation request None')
      && !status.startsWith('Simulating')
      && necromancer
      && Function('unit', `return (${predicateSource})(unit)`)(necromancer);
  }, predicate.toString(), { timeout: 30000 });
}

async function openAbility(page, name) {
  let ability = page.locator('[data-menu-key^="ability:"]').filter({ hasText: name });
  if (!(await ability.isVisible().catch(() => false))) {
    await sendAccepted(page, 'menu-job:primary');
    ability = page.locator('[data-menu-key^="ability:"]').filter({ hasText: name });
  }
  await expect(ability).toBeVisible({ timeout: 5000 });
  const text = await ability.innerText();
  const key = await ability.getAttribute('data-menu-key');
  await sendAccepted(page, `menu-${key}`);
  await page.waitForFunction(() => window.__pystralLastTransientState?.ability_targets, {
    timeout: 5000,
  });
  return text;
}

async function chooseTargetAndCommit(page, label) {
  const target = label
    ? page.locator('[data-menu-key^="target:"]').filter({ hasText: label }).first()
    : page.locator('[data-menu-key^="target:"]').first();
  await expect(target).toBeVisible({ timeout: 5000 });
  const key = await target.getAttribute('data-menu-key');
  await sendAccepted(page, `menu-target:${key.split(':')[1]}`);
  await sendAccepted(page, 'confirm');
}

test('Rhai Necromancer raises, harvests, and spends Fresh Soul on Soul Drain', async ({ page }) => {
  test.setTimeout(60000);
  await loadWithFixture(page, 'necromancer_combo');
  await page.goto('/game.html');
  await page.waitForFunction(() => window.app !== undefined, { timeout: 10000 });
  await waitForPlayerBoundary(page, {
    after: { output: -1, input: -1, history: -1 },
    unitId: 5,
  });
  await waitForNecromancer(page, (unit) => unit.mana === 0 && unit.action_points === 4);

  const raise = await openAbility(page, 'Raise Skeleton');
  expect(raise).toContain('2 AP, 20 HP (floor 1)');
  await chooseTargetAndCommit(page);
  await waitForNecromancer(page, (unit) => unit.health === 80 && unit.mana === 0 && unit.action_points === 2);
  await page.waitForFunction(() => {
    const summon = window.__pystralLastTransientState?.unit_states
      ?.find(({ unit_id }) => unit_id === 7)?.state;
    return summon?.health === 80;
  });

  await sendAccepted(page, 'wait');
  await waitForNecromancer(page, (unit) => unit.health === 80 && unit.action_points === 4);

  await openAbility(page, 'Harvest Skeleton');
  await chooseTargetAndCommit(page, 'Unit 7');
  await waitForNecromancer(page, (unit) => unit.health === 90 && unit.mana === 10 && unit.action_points === 3);
  await page.waitForFunction(() => window.__pystralLastTransientState?.unit_states
    ?.find(({ unit_id }) => unit_id === 7)?.state?.health === 0);

  const drain = await openAbility(page, 'Soul Drain');
  expect(drain).toContain('3 AP -> 2 AP (Fresh Soul), 10 MP');
  await chooseTargetAndCommit(page, 'Unit 3');
  await waitForNecromancer(page, (unit) => unit.health > 90 && unit.mana === 0 && unit.action_points === 1);

  const finalState = await page.evaluate(() => window.__pystralLastTransientState);
  const enemy = finalState.unit_states.find(({ unit_id }) => unit_id === 3).state;
  expect(enemy.health).toBeLessThan(140);
  await expect(page.locator('#action-log')).toContainText('used ability');

  const projectileLifecycle = await page.evaluate(() => {
    const events = window.__pystralActionLogEvents;
    const spawns = events.filter((event) => event.SpawnEntity?.kind === 'projectile');
    const projectileIds = spawns.map((event) => event.SpawnEntity.id);
    return {
      projectileIds,
      soulOrbs: events.filter((event) => event.UpdateProperty
        && projectileIds.includes(event.UpdateProperty.id)
        && event.UpdateProperty.property === 'asset'
        && event.UpdateProperty.value?.AssetRef === 'SoulOrb').length,
      moves: events.filter((event) => event.MoveSprite
        && projectileIds.includes(event.MoveSprite.id)).length,
      despawns: events.filter((event) => event.DespawnEntity
        && projectileIds.includes(event.DespawnEntity.id)).length,
      entityViewer: document.getElementById('entity-viewer').textContent,
    };
  });
  expect(projectileLifecycle.projectileIds).toHaveLength(1);
  expect(projectileLifecycle.soulOrbs).toBe(1);
  expect(projectileLifecycle.moves).toBe(1);
  expect(projectileLifecycle.despawns).toBe(1);
  expect(projectileLifecycle.entityViewer)
    .not.toContain(`Entity ${projectileLifecycle.projectileIds[0]} (`);
});
