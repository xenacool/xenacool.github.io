
import { chromium } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const GLB_DIR = path.join(process.cwd(), 'assets/gltf/jobs');
const OUTPUT_DIR = path.join(process.cwd(), 'assets/spritestacks');
const SPRACKER_URL = 'http://localhost:5173/';

async function main() {
  const glbFiles = fs.readdirSync(GLB_DIR).filter(f => f.endsWith('.glb'));
  console.log(`Found ${glbFiles.length} GLB files.`);

  const browser = await chromium.launch();
  const page = await browser.newPage();
  
  // Intercept requests to serve GLB files from local filesystem
  await page.route('**/local-models/**', async (route) => {
    const url = route.request().url();
    const fileName = url.split('/').pop();
    if (fileName) {
      const filePath = path.join(GLB_DIR, fileName);
      if (fs.existsSync(filePath)) {
        const body = fs.readFileSync(filePath);
        await route.fulfill({ body, contentType: 'model/gltf-binary' });
        return;
      }
    }
    await route.abort();
  });

  await page.goto(SPRACKER_URL);
  await page.waitForFunction(() => (window as any).loadGltf !== undefined);

  for (const file of glbFiles) {
    const modelName = path.basename(file, '.glb');
    console.log(`Processing ${modelName}...`);

    try {
      await page.evaluate(async ({ modelName, file }) => {
        const THREE = (window as any).THREE;
        const loadGltf = (window as any).loadGltf;
        const useModelStore = (window as any).useModelStore;
        const useLayerStore = (window as any).useLayerStore;

        const modelStore = useModelStore();
        const layerStore = useLayerStore();

        // Load GLB
        const gltf = await loadGltf(`/local-models/${file}`);
        
        // Find idle animation and seek to frame 0
        const mixer = new THREE.AnimationMixer(gltf.scene);
        const clip = THREE.AnimationClip.findByName(gltf.animations, 'idle') || gltf.animations[0];
        if (clip) {
          const action = mixer.clipAction(clip);
          action.play();
          mixer.setTime(0);
          mixer.update(0);
        }

        // Set model in store
        modelStore.setModel(gltf.scene);
        layerStore.projectName = modelName;

        // Create 300 layers at 9% thickness
        layerStore.createEvenlySpacedLayers(
          modelStore.model,
          modelStore.modelBox,
          modelStore.modelSize,
          300,
          9
        );

        // Return layer data URLs
        return layerStore.layers.map((l: any) => l.canvasDataUrl);
      }, { modelName, file });

      // Wait a bit for renders to complete if necessary
      // In Spracker, LayerService.render() returns a data URL synchronously by rendering to a hidden canvas.
      
      const layerDataUrls = await page.evaluate(() => {
          const layerStore = (window as any).useLayerStore();
          return layerStore.layers.map((l: any) => l.canvasDataUrl);
      });

      const modelOutputDir = path.join(OUTPUT_DIR, modelName);
      if (!fs.existsSync(modelOutputDir)) {
        fs.mkdirSync(modelOutputDir, { recursive: true });
      }

      for (let i = 0; i < layerDataUrls.length; i++) {
        const dataUrl = layerDataUrls[i];
        const base64Data = dataUrl.replace(/^data:image\/png;base64,/, "");
        fs.writeFileSync(path.join(modelOutputDir, `layer-${i + 1}.png`), base64Data, 'base64');
      }

      console.log(`Successfully generated ${layerDataUrls.length} layers for ${modelName}.`);
    } catch (error) {
      console.error(`Failed to process ${modelName}:`, error);
    }
  }

  await browser.close();
}

main().catch(console.error);
