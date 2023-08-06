const tabMenus = document.querySelectorAll('.tabs');

tabMenus.forEach(tabMenu => {
  const buttons = tabMenu.querySelectorAll('button');

  const selectedIndicator = document.createElement('div');
  selectedIndicator.classList.add('tabsindicator');
  selectedIndicator.style.width = `${buttons[0].offsetWidth}px`;
  tabMenu.appendChild(selectedIndicator);

  buttons.forEach(button => {
    button.addEventListener('click', () => {
      tabMenu.querySelector('.tabsindicator').style.left = `${button.offsetLeft}px`;
    });
  });
});