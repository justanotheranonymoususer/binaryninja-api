#pragma once

#include <QSortFilterProxyModel>
#include <QStandardItemModel>
#include <QTableView>

#include "filter.h"
#include "typeeditor.h"
#include "warp.h"

// A list of a functions possible matches.

class WARPMatchesFilterModel;
class WARPMatchedFunctionItemModel;
class WARPMatchesTableView;

class WARPMatchesWidget : public QWidget, public FilterTarget
{
    Q_OBJECT
    FunctionRef m_current;

    QSplitter* m_splitter;

    FilterEdit* m_filterEdit;
    FilteredView* m_filterView;

    WARPMatchesTableView* m_matchesTable;
    WARPMatchedFunctionItemModel* m_matchesModel;
    WARPMatchesFilterModel* m_proxyModel;
public:
    explicit WARPMatchesWidget(FunctionRef current);
    ~WARPMatchesWidget() override = default;
    void SetCurrentFunction(FunctionRef current) { m_current = current; };
    FunctionRef GetCurrentFunction() { return m_current; };
    void UpdateMatches();

    void setFilter(const std::string&) override;
    void scrollToFirstItem() override {}
    void scrollToCurrentItem() override {}
    void selectFirstItem() override {}
    void activateFirstItem() override {}
};

class WARPMatchesFilterModel: public QSortFilterProxyModel
{
    Q_OBJECT

public:
    WARPMatchesFilterModel(QObject* parent): QSortFilterProxyModel(parent) { }
    ~WARPMatchesFilterModel() override = default;

    void notifyFilterParametersChanged();
    bool filterAcceptsRow(int sourceRow, const QModelIndex& sourceParent) const override;
    bool lessThan(const QModelIndex& sourceLeft, const QModelIndex& sourceRight) const override;
};

class WARPMatchedFunctionItemModel : public QStandardItemModel
{
    Q_OBJECT
    QVector<QList<QStandardItem*>> m_rows;

public:
    // TODO: This is so bad.
    BNWARPFunction** m_functions = nullptr;
    size_t m_functionCount = 0;

    WARPMatchedFunctionItemModel(const QStringList& labels, QObject* parent);
    void AddRow(QList<QStandardItem*>& row) { m_rows.push_back(row); }
    void Clear()
    {
        // Free i guess lol
        if (m_functions)
        {
            BNWARPFreeFunctionList(m_functions, m_functionCount);
        }
        m_rows.clear();
    }
    void Refresh();
};

class WARPMatchesTableView: public QTableView
{
    Q_OBJECT
    WARPMatchedFunctionItemModel* m_model = nullptr;

public:
    WARPMatchesTableView(const QStringList& labels, QWidget* parent);
    WARPMatchedFunctionItemModel* GetModel() { return m_model; }
};