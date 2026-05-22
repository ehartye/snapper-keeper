/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: 'var(--bg)',
        'bg-soft': 'var(--bg-soft)',
        surface: 'var(--surface)',
        'surface-2': 'var(--surface-2)',
        border: 'var(--border)',
        fg: 'var(--text)',
        'fg-muted': 'var(--text-muted)',
        primary: 'var(--primary)',
        accent: 'var(--accent)',
        'accent-2': 'var(--accent-2)',
        'accent-3': 'var(--accent-3)',
        'accent-4': 'var(--accent-4)',
        danger: 'var(--danger)',
      },
      fontFamily: {
        display: 'var(--display-font)',
        body: 'var(--body-font)',
      },
    },
  },
  plugins: [],
};
