import React, { Component } from 'react';
import classnames from 'classnames';
import { Link } from 'react-router';
import render from '../render';
import {get_login} from '../login'

import "./login.css"

export default class Login extends Component {

  setLogin(event) {
    document.cookie = "swindon_muc_login=" + event.target.value;
    render()
  }
  render() {
    const { className } = this.props;
    const login = get_login();
    return (
      <div className={ classnames('Login', className) }>
        <h1>
          Sign-in
        </h1>
        <input type="text" placeholder="Your Name"
          onInput={this.setLogin}
          value={login} />
        {login && <Link to="/">
                    <button>Sign in</button>
                  </Link>}
      </div>
    );
  }
}
