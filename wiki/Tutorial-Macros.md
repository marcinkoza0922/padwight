# Tutorial: record a sequence into a macro

A macro is a saved sequence of presses, holds, releases and waits, played with one button. You'll make a *Quick turn* macro and put it on a button.

## 1. Make the macro

![The Macros tab with a Quick turn macro of three steps and the add-step buttons](https://raw.githubusercontent.com/marcinkoza0922/padwight/main/docs/images/manual/macros-tab.png "Quick turn: tap Left Shift, wait, tap the right mouse button.")

1. Open the setup's **Macros** tab and press **+ New macro** (1). The new macro opens at the top, ready to name.
2. Type its name in the **Name** box: *Quick turn*.
3. Add steps with the buttons under the list (3):
   - **Tap** presses and lets go after **Held for** milliseconds.
   - **Hold down** keeps holding until a **Release** of the same input, or the end of the macro.
   - **Release** lets go of something held.
   - **Wait** pauses.
   - **Move stick** puts a virtual stick somewhere until another step moves it.
   - **Insert motion…** adds a fighting-game motion (quarter circle, dragon punch or half circle) for a character facing right.
4. Each step's action chooses the input: keys, a mouse button, the scroll wheel or a gamepad button. In the picture (2), step 1 taps **Left Shift** for 40 ms, step 2 waits 50 ms, and step 3 taps the right mouse button.
5. The arrows beside a step move it, and **×** removes it. The macro's header shows its steps and how long it plays.

## 2. Put it on a button

1. On a profile's **Buttons** tab, open the button you want, say **RB**.
2. Click its action, choose **Macro…** and pick *Quick turn*.
3. Choose how it plays:
   - **Play once** (the default): each press plays the whole macro, even if you let go early.
   - **Repeat while held** loops it while you hold the button and stops straight away when you let go.

Anything the macro still holds is let go when a pass ends. To loop a macro without holding the button, choose **Toggle on / off…** and put the macro inside it.

## 3. Save and try it

Press **Save & apply** and press the button. Renaming a macro updates every button that uses it, and a button whose macro is missing says so.

## Where next

- [Macros](Macros) for the reference.
- Menu items can run macros too: see [the menus tutorial](Tutorial-Menus).
