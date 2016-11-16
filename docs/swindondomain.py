from sphinxcontrib.domaintools import custom_domain


def setup(app):
    app.add_domain(custom_domain('SwindonConfig',
        name  = 'swindon',
        label = "Swindon Config",

        elements = dict(
            opt = dict(
                objname      = "Configuration Option",
                indextemplate = "pair: %s; Config Option",
            ),
            sect = dict(
                objname      = "Configuration Section",
                indextemplate = "pair: %s; Config Section",
            ),
            handler = dict(
                objname       = "Handler",
                indextemplate = "pair: %s; Request Handler",
            ),
        )))
