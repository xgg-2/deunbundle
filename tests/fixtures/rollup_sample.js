// Rollup IIFE bundle sample fixture
// This simulates a minimal Rollup bundle for testing purposes
(function() {
  'use strict';

  var VERSION = '1.0.0';
  var APP_NAME = 'TestApp';

  function add(a, b) {
    return a + b;
  }

  function multiply(a, b) {
    return a * b;
  }

  function formatResult(value) {
    return 'Result: ' + String(value);
  }

  var MathUtils = {
    add: add,
    multiply: multiply
  };

  function Button(props) {
    return { type: 'button', props: props };
  }

  function App(config) {
    var el = document.createElement('div');
    el.textContent = APP_NAME + ' v' + VERSION;
    var sum = MathUtils.add(config.a, config.b);
    el.setAttribute('data-result', String(sum));
    return el;
  }

  var app = new App({ a: 10, b: 20 });
  document.body.appendChild(app);

  console.log(formatResult(MathUtils.multiply(3, 7)));
})();
