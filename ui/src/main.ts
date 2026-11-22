import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';

function applyCurrentTheme() {
  const savedTheme = localStorage.getItem('peek_theme');
  if (savedTheme === 'dark') {
    document.documentElement.dataset.theme = 'dark';
  } else if (savedTheme === 'light') {
    document.documentElement.dataset.theme = 'light';
  } else {
    delete document.documentElement.dataset.theme;
  }

  const savedAccent = localStorage.getItem('peek_accent');
  if (savedAccent) {
    document.documentElement.dataset.accent = savedAccent;
  }
}

applyCurrentTheme();
window.addEventListener('storage', applyCurrentTheme);

const target = document.getElementById('app');
if (target) {
  mount(App, { target });
}
