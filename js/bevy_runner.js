/**
 * Bevy WASM Runner for mdBook inline examples
 *
 * Loads and initializes Bevy WASM modules into <canvas> elements.
 * Uses IntersectionObserver for lazy initialization when canvases scroll into view.
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

    // Base URL for WASM examples
    const EXAMPLES_BASE = 'demos/examples/';

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
     * Load a WASM module dynamically.
     */
    async function loadWasmModule(lib, example, canvas) {
        const wasmPath = `${EXAMPLES_BASE}${lib}/${example}/${example}.js`;
        const wasmUrl = new URL(wasmPath, window.location.href).href;

        // Dynamically import the wasm-bindgen JS file
        const module = await import(/* webpackIgnore: true */ wasmUrl);
        return module;
    }

    /**
     * Initialize a single canvas element with its WASM example.
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

        try {
            // Load the WASM module
            const module = await loadWasmModule(lib, example, canvas);

            // Initialize the WASM module with the canvas
            // The wasm-bindgen generated JS exports an init function
            await module.default(canvas);

            // Remove loading spinner
            if (spinner && spinner.parentElement) {
                spinner.remove();
            }
            canvas.classList.remove('bevy-loading');
            canvas.classList.add('bevy-initialized');

            // Track this canvas for cleanup
            initializedCanvases.set(canvas, { lib, example, module });

            console.log(`BevyRunner: Initialized ${lib}/${example}`);
        } catch (error) {
            console.error(`BevyRunner: Failed to load ${lib}/${example}:`, error);

            // Remove loading spinner
            if (spinner && spinner.parentElement) {
                spinner.remove();
            }
            canvas.classList.remove('bevy-loading');

            // Determine error message
            let message = 'Failed to load this example.';
            if (error.message && error.message.includes('404')) {
                message = 'Example WASM not found. It may not be built yet.';
            } else if (error.message && error.message.includes('WebAssembly')) {
                message = 'WebAssembly is not supported in your browser.';
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
            // Stop the render loop if possible
            if (data.module && data.module.stop) {
                data.module.stop();
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
