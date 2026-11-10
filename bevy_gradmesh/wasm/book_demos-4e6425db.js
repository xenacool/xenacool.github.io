import('/js/book_demos/main.js').catch((error) => {
  console.warn(`Interactive demos failed to load: ${error.message}`);
});
