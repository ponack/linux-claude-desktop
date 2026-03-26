<script>
  let { content = "" } = $props();

  let srcdoc = $derived(`<!DOCTYPE html>
<html>
<head>
  <script src="https://unpkg.com/react@18/umd/react.development.js" crossorigin><\/script>
  <script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js" crossorigin><\/script>
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"><\/script>
  <style>
    body { margin: 0; padding: 16px; font-family: system-ui, sans-serif; background: #fff; color: #333; }
    #root { min-height: 100vh; }
    .error { color: #e94560; padding: 16px; font-family: monospace; white-space: pre-wrap; }
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="text/babel">
    try {
      ${content.replace(/<\/script>/g, '<\\/script>')}

      // Try to find and render the default export or last component
      const componentNames = Object.keys(window).filter(k => /^[A-Z]/.test(k) && typeof window[k] === 'function');
      let Component;
      try {
        // Prefer if there's an explicit App or default export pattern
        Component = typeof App !== 'undefined' ? App : componentNames.length > 0 ? window[componentNames[componentNames.length - 1]] : null;
      } catch(e) { Component = null; }

      if (Component) {
        ReactDOM.createRoot(document.getElementById('root')).render(React.createElement(Component));
      }
    } catch (e) {
      document.getElementById('root').innerHTML = '<div class="error">Error: ' + e.message + '</div>';
    }
  <\/script>
</body>
</html>`);
</script>

<iframe
  {srcdoc}
  sandbox="allow-scripts"
  title="React component preview"
  class="react-iframe"
></iframe>

<style>
  .react-iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: #ffffff;
  }
</style>
