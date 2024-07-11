#ifndef GOGHVIEWER_H
#define GOGHVIEWER_H

#include <QMainWindow>
#include <QGraphicsScene>
#include <QPixmap>

QT_BEGIN_NAMESPACE
namespace Ui
{
    class GoghViewer;
}
QT_END_NAMESPACE

class GoghViewer : public QMainWindow
{
    Q_OBJECT

public:
    GoghViewer(QWidget *parent = nullptr);
    ~GoghViewer();
    void openImage(QString selectedFile);

private slots:

    void on_action_Open_triggered();

    void on_btnOpenFile_clicked();

    void on_actionAbout_triggered();

    void on_pushButton_clicked();

    void on_btnToolGreyscale_clicked();

    void on_btnSaveAs_clicked();

    void on_dialHueRotate_valueChanged(int value);

private:
    Ui::GoghViewer *ui;
    QPixmap pm;
    std::unique_ptr<QGraphicsScene> scene;
    void openImage();
    QPixmap loadImage(QString imagePath);
    void setShownImage(QPixmap pixmap);
};
#endif // GOGHVIEWER_H
