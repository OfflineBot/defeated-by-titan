
# Defeated by Titan

## Description

This is the first prompt of this project. 
This markdown defines standards, workflows and how to keep implementing on a bigger project. 
There are examples of crates/concepts you dont have to use them directly. Those are examples. They are here to explain the concept!

What is missing: The core roadmap or direct information about the game. This is only about structure. 

*Definition of done (for the full project, not this file):* I have a polished, playable version of the game how i want it to be. No placeholders, no bugs, clear coding structure and easy to extend. All features must be implemented and playtested.

## Initialize the project
Collect all informations and create the files for you to remember this. Extrate this markdown and put it in your workspace. Maybe create a docs folder or better an .ai folder. also create the README.md to link to that folder for future sessions. 
Then analyse the full context about the game and directly ask questions if you dont understand a system or concept. Write the full project compact for you down so you will find the information later in new sessions. Make it easy and fast to find and the most important: Stick to your conventions. Where files are for you to find to not waste any tokens. Define them. Conventinos are super important. not only in code!

When you are fully done extracting all the information into you new workspace delete this markdown and begin with the rust/bevy setup and implementatino of the game. 

## Ai working behaviour

### Coding / Implementation
If ai is not sure in any sense it *must* ask the user. This is a hard rule to prevent halucination and unclear implementation of systems. 
If the ai thinks, a tool or implemntation (etc) could be better *ask* first before implementing it. 
Golden rule here is: *Never assume without user input*

Dont waste tokens. There are enough but there is no need to waste them.
Also self optimize. if there are major mistakes also try to find the problem to not do it again.

Self optimize. If the project slows down. find the problems, fix documentation you write. Dont make the documentation too large. It will outdate too fast. 

### Agents
If allowed, use agents but not use them because you were told to. Use them in a useable sense. If there is one problem, multiple agents make it worse than one. 
If the user demands multiple agents remind him.

## Code

### Code Structure
Code with the yagni principle. Reuse functions. Keep a useable code structure.

Everything besides the main entry point is a crate. 
Entites are a crate by its own. 
Movement is a crate and so on.

Crates all export a <Cratename>Plugin which is used in the main as plugin.

If a crate like entity needs movement it is allowed to use the movement. But then its no longer allowed for movement to implement entity. (No circular dependencies (just like code imports)).
This should make each system modular and better to debug. 

A system is a domain of a specific field. entities dont hold inputs, they come from another crate and so on.

When a module is really bad sometimes its better to rewrite it from scratch. You are allowed to do that. Quality over code density. Quality over time.

Example structure:

crates/
    movement/
        src/
            lib.rs
        Cargo.toml
    entities/
        src/
            lib.rs
        Cargo.toml
    asset_loader/
        src/
            lib.rs
        Cargo.toml
game/
    src/
        main.rs
        Cargo.toml
Cargo.toml

### Code Convenctions
Any bevy component that is just a Marker like the main camera marker ends with `*Marker`.
As soon it holds information it is no marker anymore!

Events/Messages end all with `*Msg`

Systems dont just do something because they cant with the current project structure. They have intents. Like a movement intent where they say: "i want to go there" and the movement system decides what happens then! 

EVERYTHING is in english. comments, code, documentation. Everything!

### Code iterations
Dont implement placeholder systems. For example no pseudo movement system to test hitboxes and later replace the full movement system with the correct one. Directly implement the stuff and make it modular.
If the player requires a asset, dont just make a short sloppy asset loading/handling system. Directly implement the core asset system to be used and extended later!
This should prevent dead code.

