/**
 * Bevy WASM Runner for mdBook inline examples
 *
 * Loads a single shared WASM runtime containing all examples.
 * Each canvas gets a unique ID so the shared runtime can dispatch
 * to the correct example via the ?example= parameter.
 *
 * Each canvas element should have:
 *   data-lib="bevy_gradmesh" - the library name
 *   data-example="shapes" - the example name
 *   data-width="1280" - original width
 *   data-height="720" - original height
 */

const BevyRunner = (function() {
    'use strict';

    // Track all initialized canvases for cleanup
    const initializedCanvases = new Map();

    // Shared WASM runtime path (relative to book pages)
    let wasmRuntime = null;
    let wasmPromise = null;

    /**
     * Get the relative path to the shared WASM runtime.
     * Adjusts based on current page depth (root vs /wasm/ subdirectory).
     */
    function getWasmPath() {
        const path = window.location.pathname;
        if (path.includes('/wasm/')) {
            // We're in /bevy_gradmesh/wasm/ or /bevy_composable_dragging/wasm/
            return '../../demos/wasm/wasm_runner.js';
        }
        // We're in /bevy_gradmesh/ or /bevy_composable_dragging/
        return '../demos/wasm/wasm_runner.js';
    }

    /**
     * Create a loading spinner overlay for a canvas container.
     */
    function createLoadingSpinner(container) {
        const spinner = document.createElement('div');
        spinner.className = 'bevy-loading-spinner';
        spinner.innerHTML = `
            <div class="bevy-spinner"></div>
            <div class="bevy-loading-text">Loading example...</div>
        `;
        container.appendChild(spinner);
        return spinner;
    }

    /**
     * Create an error message overlay for a canvas container.
     */
    function createErrorMessage(container, message, sourceLink) {
        const errorDiv = document.createElement('div');
        errorDiv.className = 'bevy-error-message';
        errorDiv.innerHTML = `
            <div class="bevy-error-icon">⚠️</div>
            <div class="bevy-error-text">${message}</div>
            ${sourceLink ? `<a href="${sourceLink}" target="_blank" class="bevy-source-link">View source code</a>` : ''}
        `;
        container.appendChild(errorDiv);
        return errorDiv;
    }

    /**
     * Check if WebGL2 is supported.
     */
    function checkWebGL2Support() {
        try {
            const canvas = document.createElement('canvas');
            return !!canvas.getContext('webgl2');
        } catch (e) {
            return false;
        }
    }

    /**
     * Load the shared WASM runtime once.
     */
    async function loadSharedRuntime() {
        if (wasmPromise) {
            return wasmPromise;
        }

        const wasmPath = getWasmPath();
        const wasmUrl = new URL(wasmPath, window.location.href).href;

        wasmPromise = fetch(wasmUrl)
            .then(response => {
                if (!response.ok) {
                    throw new Error(`WASM runtime not found (${response.status})`);
                }
                return response.text();
            })
            .then(text => {
                // Execute the wasm-bindgen JS in a scope that captures the init function
                const moduleExports = {};
                const _exports = {};
                // Create a fake module object for the wasm-bindgen code
                const mod = { exports: moduleExports };
                // Evaluate the JS with the right scope
                const fn = new Function('module', 'exports', 'window', 'globalThis', text);
                fn(mod, moduleExports, window, typeof globalThis !== 'undefined' ? globalThis : window);
                wasmRuntime = moduleExports;
                return moduleExports;
            });

        return wasmPromise;
    }

    /**
     * Initialize a single canvas element with the shared WASM runtime.
     */
    async function init(canvas) {
        const lib = canvas.dataset.lib;
        const example = canvas.dataset.example;

        if (!lib || !example) {
            console.error('BevyRunner: Canvas missing data-lib or data-example attribute');
            return;
        }

        // Check for WebGL2 support
        if (!checkWebGL2Support()) {
            const container = canvas.parentElement;
            createErrorMessage(container, 'WebGL2 is not supported in your browser.', null);
            canvas.style.display = 'none';
            return;
        }

        // Add loading state
        const container = canvas.parentElement;
        const spinner = createLoadingSpinner(container);
        canvas.classList.add('bevy-loading');

        // Give this canvas a unique ID for Bevy to target
        const canvasId = `bevy-canvas-${example}-${Math.random().toString(36).substr(2, 6)}`;
        canvas.id = canvasId;

        try {
            // Load the shared WASM runtime
            const mod = await loadSharedRuntime();

            // The wasm-bindgen init function loads the WASM and returns a promise
            // We need to set the example name before init by modifying the URL search
            // Since the WASM reads ?example= from the URL, we can't easily do that per-canvas.
            // Instead, we'll use the init function directly.
            
            // For the shared runtime approach, we need to call init() which starts Bevy
            // The Bevy app reads ?example= from the URL, but we have multiple canvases.
            // Solution: each canvas gets its own hidden iframe that loads the WASM with
            // the correct ?example= parameter.
            
            // Actually, the simplest approach: create a hidden iframe per canvas
            const iframe = document.createElement('iframe');
            iframe.style.display = 'none';
            iframe.style.width = '0';
            iframe.style.height = '0';
            iframe.style.border = 'none';
            
            // Build the iframe src with the example parameter
            const wasmDir = new URL(getWasmPath(), window.location.href).href.replace(/wasm_runner\.js$/, '');
            const iframeSrc = `${wasmDir}demo-frame.html?example=${example}`;
            iframe.src = iframeSrc;
            
            // Wait for the iframe to load, then steal its canvas
            await new Promise((resolve, reject) => {
                const timeout = setTimeout(() => {
                    reject(new Error('Example timed out loading'));
                }, 30000);
                
                iframe.onload = () => {
                    clearTimeout(timeout);
                    // Poll for Bevy to create its canvas
                    let attempts = 0;
                    const poll = () => {
                        try {
                            const innerDoc = iframe.contentDocument || iframe.contentWindow.document;
                            const innerCanvas = innerDoc.querySelector('canvas#bevy-canvas');
                            if (innerCanvas && innerCanvas.getContext) {
                                // Transfer the canvas to our container
                                innerCanvas.id = canvasId;
                                innerCanvas.className = 'bevy-example bevy-initialized';
                                innerCanvas.style.width = canvas.style.width || '100%';
                                innerCanvas.style.height = canvas.style.height || 'auto';
                                innerCanvas.style.maxWidth = canvas.style.maxWidth || '100%';
                                
                                // Replace our placeholder canvas with the real one
                                canvas.replaceWith(innerCanvas);
                                
                                // Track for cleanup
                                initializedCanvases.set(innerCanvas, { lib, example, iframe });
                                
                                // Remove loading spinner
                                if (spinner && spinner.parentElement) {
                                    spinner.remove();
                                }
                                
                                console.log(`BevyRunner: Initialized ${lib}/${example}`);
                                resolve();
                                return;
                            }
                        } catch (e) {
                            // Cross-origin error, ignore
                        }
                        attempts++;
                        if (attempts < 100) {
                            setTimeout(poll, 100);
                        } else {
                            reject(new Error('Canvas not created by Bevy'));
                        }
                    };
                    poll();
                };
                
                iframe.onerror = () => {
                    clearTimeout(timeout);
                    reject(new Error('Failed to load example iframe'));
                };
                
                container.appendChild(iframe);
            });

        } catch (error) {
            console.error(`BevyRunner: Failed to load ${lib}/${example}:`, error);

            // Remove loading spinner
            if (spinner && spinner.parentElement) {
                spinner.remove();
            }
            canvas.classList.remove('bevy-loading');

            // Determine error message
            let message = 'Failed to load this example.';
            if (error.message && error.message.includes('not found')) {
                message = 'Example WASM not found. It may not be built yet.';
            } else if (error.message && error.message.includes('timed out')) {
                message = 'Example took too long to load.';
            }

            // Create source link
            const sourceLink = `https://github.com/xenacool/${lib}/tree/main/examples/${example}.rs`;

            createErrorMessage(container, message, sourceLink);
            canvas.style.display = 'none';
        }
    }

    /**
     * Destroy a canvas element and clean up resources.
     */
    function destroy(canvas) {
        const data = initializedCanvases.get(canvas);
        if (data) {
            console.log(`BevyRunner: Destroying ${data.lib}/${data.example}`);
            if (data.iframe && data.iframe.parentElement) {
                data.iframe.remove();
            }
            initializedCanvases.delete(canvas);
            canvas.classList.remove('bevy-initialized');
        }
    }

    /**
     * Set up IntersectionObserver for lazy initialization.
     */
    function observe() {
        // Check if IntersectionObserver is available
        if (!('IntersectionObserver' in window)) {
            console.warn('BevyRunner: IntersectionObserver not available, initializing all canvases immediately');
            document.querySelectorAll('canvas.bevy-example').forEach(canvas => {
                if (!initializedCanvases.has(canvas)) {
                    init(canvas);
                }
            });
            return;
        }

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    const canvas = entry.target;
                    if (!initializedCanvases.has(canvas)) {
                        console.log(`BevyRunner: Canvas visible, initializing ${canvas.dataset.lib}/${canvas.dataset.example}`);
                        init(canvas);
                    }
                    // Stop observing once initialized
                    observer.unobserve(canvas);
                }
            });
        }, {
            rootMargin: '100px', // Start loading 100px before visible
            threshold: 0.1
        });

        // Observe all bevy-example canvases
        document.querySelectorAll('canvas.bevy-example').forEach(canvas => {
            if (!initializedCanvases.has(canvas)) {
                observer.observe(canvas);
            }
        });
    }

    /**
     * Clean up all initialized canvases when page is unloaded.
     */
    function cleanup() {
        initializedCanvases.forEach((data, canvas) => {
            destroy(canvas);
        });
    }

    // Set up cleanup on page unload
    if (typeof window !== 'undefined') {
        window.addEventListener('beforeunload', cleanup);
    }

    // Public API
    return {
        init,
        destroy,
        observe,
        cleanup,
        checkWebGL2Support
    };
})();

// Auto-initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => BevyRunner.observe());
} else {
    BevyRunner.observe();
}
