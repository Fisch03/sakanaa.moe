const tabMenus = document.querySelectorAll('.tabs');

window.addEventListener('resize', buildTabs);

function buildTabs() {
  tabMenus.forEach(tabMenu => {
    let vertical = tabMenu.classList.contains('vertical');
  
    const buttons = tabMenu.querySelectorAll('button, .navbutton');
  
  
    let selectedIndicator = tabMenu.querySelector('.tabsindicator');
    if(!selectedIndicator) {
      selectedIndicator = document.createElement('div');
      selectedIndicator.classList.add('tabsindicator');
      tabMenu.appendChild(selectedIndicator);
    }
  
    if(!vertical) {
      selectedIndicator.style.width = `${buttons[0].offsetWidth}px`;
    } else {
      selectedIndicator.style.height = `${buttons[0].offsetHeight}px`;
    }
  
    buttons.forEach(button => {
      button.addEventListener('click', () => {
        if(!vertical) {
          tabMenu.querySelector('.tabsindicator').style.left = `${button.offsetLeft}px`;
        } else {
          tabMenu.querySelector('.tabsindicator').style.top = `${button.offsetTop}px`;
        }
      });
    });
  });
}

buildTabs();

const navigation = document.querySelector('#Navigation');
const navigationMenu = navigation.querySelector('.sectioncontent');
const navigationButtons = navigation.querySelectorAll('.sectioncontent > button');
let buttonObservers = [];

let isScrolling = false;
let scrollTimeout = null;

navigationButtons.forEach(button => {
  button.addEventListener('click', () => {
    const target = button.dataset.target;
    document.querySelector(`#${target}`).scrollIntoView({behavior:'smooth',block:'center'});
    isScrolling = true;
    console.log('scrolling started');
  });

  buttonObservers.push(new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if(entry.isIntersecting && !isScrolling) {
        navigationMenu.querySelector('.tabsindicator').style.top = `${button.offsetTop}px`;
      }
    });
  }, {threshold: 1}).observe(document.querySelector(`#${button.dataset.target}`)));
});

window.addEventListener('scroll', () => {
  if(!isScrolling) return;
  console.log('scrolling');
  clearTimeout(scrollTimeout);
  scrollTimeout = setTimeout(() => {
    isScrolling = false;
    console.log('scrolling stopped');
  }, 100);
});