# Building

The first step is getting Obliteration source code from our [repository](https://github.com/obhq/obliteration). If you plan to make some contributions you need to fork our repository by clicking on Fork on the top right before proceed and all instructions after this need to be done on your forked repository instead of our repository.

Once ready click on Code button on the top right then copy the HTTPS URL from the popup (or SSH if you know how to use it). Then open a terminal (or Command Prompt) on the directory where you want to download the source code into. You can use `cd` command to change to the directory you want. Then run the following command:

```sh
git clone URL
```

Replace `URL` with the URL you have copied.

## Build

Change into the directory you just downloaded with `cd` and run the following command:

```sh
project build -r
```

Remove `-r` to disable optimization if you plan to make some contributions so the debugger will work properly. Build outputs will be placed in `dist` directory.
