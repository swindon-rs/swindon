import React, { Component } from 'react';
import classnames from 'classnames';
import { IndexLink } from 'react-router';


export default class Room extends Component {

  setLogin(event) {
    document.cookie = "swindon_muc_login=" + event.target.value;
  }
  render() {
    const { className, login } = this.props;
    return (
      <div className={ classnames('Login', className) }>
        <h1>
          Sign-in
        </h1>
        <input type="text" placeholder="Your Name"
          onInput={this.setLogin}
          value={login} />
        <IndexLink to="/">
          <button>Sign in</button>
        </IndexLink>
      </div>
    );
  }
}
